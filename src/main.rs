use anyhow::Context;
use cardamon::{
    carbon_intensity::{fetch_ci, fetch_region_code, valid_region_code, GLOBAL_CI},
    cleanup_stdout_stderr,
    config::{self, Config, ExecutionPlan, ProcessToObserve},
    data::{dataset::LiveDataFilter, Data},
    db_connect, db_migrate, init_config,
    models::rab_model,
    run, server, stats::{stats_output_json, stats_output_text},
};
use chrono::Utc;
use clap::{arg, Parser, Subcommand, ValueEnum};
use colored::Colorize;
use dotenvy::dotenv;
use itertools::Itertools;
use std::{env, path::Path};
use term_table::{row, row::Row, rows, table_cell::*, Table, TableStyle};
use tracing_subscriber::EnvFilter;
// use textplots::{AxisBuilder, Chart, Plot, Shape, TickDisplay, TickDisplayBuilder};

#[derive(Parser, Debug)]
#[command(author = "Oliver Winks (@ohuu), William Kimbell (@seal)", version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub file: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum StatsOutputFormat {
    #[value(alias("text"))]
    Text,

    #[value(alias("json"))]
    Json,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Runs a single observation")]
    Run {
        #[arg(help = "Please provide an observation name")]
        name: String,

        #[arg(value_name = "REGION", short, long)]
        region: Option<String>,

        #[arg(value_name = "EXTERNAL PIDs", short, long, value_delimiter = ',')]
        pids: Option<Vec<String>>,

        #[arg(
            value_name = "EXTERNAL CONTAINER NAMES",
            short,
            long,
            value_delimiter = ','
        )]
        containers: Option<Vec<String>>,

        #[arg(long)]
        external_only: bool,
    },

    Stats {
        #[arg(
            help = "Please provide a scenario name ('live_<observation name>' for live monitor data)"
        )]
        scenario_name: Option<String>,
        
        #[arg(value_name = "NUMBER OF PREVIOUS", short = 'n')]
        previous_runs: Option<u64>,
        #[arg(value_enum, default_value_t=StatsOutputFormat::Text, short ='o')]
        output: StatsOutputFormat,
    },

    #[command(about = "Start the Cardamon UI server")]
    Ui {
        #[arg(short, long)]
        port: Option<u32>,
    },

    #[command(about = "Wizard for creating a cardamon.toml file")]
    Init,
}

fn load_config(file: &Option<String>) -> anyhow::Result<Config> {
    // Initialize config if it exists
    match file {
        Some(path) => {
            println!("> using config {}", path.green());
            config::Config::try_from_path(Path::new(path))
        }
        None => {
            println!("> using config {}", "./cardamon.toml".green());
            config::Config::try_from_path(Path::new("./cardamon.toml"))
        }
    }
}

fn add_external_processes(
    pids: Option<Vec<String>>,
    containers: Option<Vec<String>>,
    exec_plan: &mut ExecutionPlan,
) -> anyhow::Result<()> {
    // add external processes to observe.
    for pid in pids.unwrap_or_default() {
        let pid = pid.parse::<u32>()?;
        println!("> including external process {}", pid.to_string().green());
        exec_plan.observe_external_process(ProcessToObserve::ExternalPid(pid));
    }
    if let Some(container_names) = containers {
        exec_plan.observe_external_process(ProcessToObserve::ExternalContainers(container_names));
    }

    Ok(())
}

async fn get_or_validate_region_code(region_code: Option<String>) -> Option<String> {
    match region_code {
        None => {
            print!("> fetching region from IP address");
            match fetch_region_code().await {
                Err(err) => {
                    println!("\t{}", "✗".red());
                    println!("\t{}", format!("- {}", err).bright_black());
                    None
                }

                Ok(code) => {
                    println!("\t{}", "✓".green());
                    println!(
                        "\t{}",
                        format!("- using region code {}", code).bright_black()
                    );
                    Some(code)
                }
            }
        }

        Some(code) => {
            print!("> validating region code");
            if valid_region_code(&code) {
                println!("\t{}", "✓".green());
                Some(code)
            } else {
                println!("\t{}", "✗".red());
                None
            }
        }
    }
}

async fn get_carbon_intensity(region_code: &Option<String>) -> f64 {
    let now = Utc::now();
    match region_code {
        Some(code) => {
            print!("> fetching carbon intensity for {}", code);
            match fetch_ci(&code, &now).await {
                Ok(ci) => {
                    println!("\t{}", "✓".green());
                    println!(
                        "\t{}",
                        format!("- using {:.3} gWh CO2eq", ci).bright_black()
                    );
                    ci
                }

                Err(_) => {
                    println!("\t{}", "✗".red());
                    println!("\t{}", "- using global avg 0.494 gWh CO2eq".bright_black());
                    GLOBAL_CI
                }
            }
        }

        None => {
            print!(
                "> using global avg carbon intensity {} gWh CO2eq",
                "0.494".green()
            );
            GLOBAL_CI
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // read .env file if it exists
    dotenv().ok();

    // Parse clap args
    let args = Cli::parse();

    let log_filter = env::var("LOG_FILTER").unwrap_or("warn".to_string());

    // Set up tracing subscriber
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(log_filter))
        .with_target(false)
        // .compact()
        .pretty()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // connect to the database and run migrations
    let database_url =
        env::var("DATABASE_URL").unwrap_or("sqlite://cardamon.db?mode=rwc".to_string());
    let database_name = env::var("DATABASE_NAME").unwrap_or("".to_string());
    let db_conn = db_connect(&database_url, Some(&database_name)).await?;
    db_migrate(&db_conn).await?;

    match args.command {
        Commands::Init => {
            init_config().await;
        }

        Commands::Run {
            name,
            region,
            pids,
            containers,
            external_only,
        } => {
            let config = load_config(&args.file)
                .context("Error loading configuration, please run `cardamon init`")?;

            // get the carbon intensity
            let region_code = get_or_validate_region_code(region).await;
            let ci = get_carbon_intensity(&region_code).await;

            // create an execution plan
            let cpu = config.cpu.clone();
            let mut execution_plan = config.create_execution_plan(cpu, &name, external_only)?;

            // add external processes to observe.
            add_external_processes(pids, containers, &mut execution_plan)?;

            // Cleanup previous runs stdout and stderr
            cleanup_stdout_stderr()?;

            // run it!
            let observation_dataset_rows = run(execution_plan, &region_code, ci, &db_conn).await?;
            let observation_dataset = observation_dataset_rows
                .last_n_runs(5)
                .all()
                .build(&db_conn)
                .await?;

            println!("\n{}", " Summary ".reversed().green());
            for scenario_dataset in observation_dataset
                .by_scenario(LiveDataFilter::ExcludeLive)
                .iter()
            {
                let run_datasets = scenario_dataset.by_run();

                // execute model for current run
                let (head, tail) = run_datasets
                    .split_first()
                    .expect("Dataset does not include recent run.");
                let run_data = head.apply_model(&db_conn, &rab_model).await?;

                // execute model for previous runs and calculate trend
                let mut tail_data = vec![];
                for run_dataset in tail {
                    let run_data = run_dataset.apply_model(&db_conn, &rab_model).await?;
                    tail_data.push(run_data.data);
                }
                let tail_data = Data::mean(&tail_data.iter().collect_vec());
                let trend = run_data.data.pow - tail_data.pow;
                let trend_str = match trend.is_nan() {
                    true => "--".bright_black(),
                    false => {
                        if trend > 0.0 {
                            format!("↑ {:.3}Wh", trend.abs()).red()
                        } else {
                            format!("↓ {:.3}Wh", trend).green()
                        }
                    }
                };

                println!("{}:", scenario_dataset.scenario_name().to_string().green());

                let table = Table::builder()
                    .rows(rows![
                        row![
                            TableCell::builder("Region").build(),
                            TableCell::builder("Duration (s)".bold()).build(),
                            TableCell::builder("Power (Wh)".bold()).build(),
                            TableCell::builder("CI (gWh)".bold()).build(),
                            TableCell::builder("CO2 (g)".bold()).build(),
                            TableCell::builder(format!("Trend (over {} runs)", tail.len()).bold())
                                .build()
                        ],
                        row![
                            TableCell::new(format!(
                                "{}",
                                run_data.region.clone().unwrap_or_default()
                            )),
                            TableCell::new(format!("{:.3}s", run_data.duration())),
                            TableCell::new(format!("{:.3}Wh", run_data.data.pow)),
                            TableCell::new(format!("{:.3}gWh", run_data.ci)),
                            TableCell::new(format!("{:.3}g", run_data.data.co2)),
                            TableCell::new(trend_str)
                        ]
                    ])
                    .style(TableStyle::rounded())
                    .build();

                println!("{}", table.render())
            }
        }

        Commands::Stats {
            scenario_name,
            previous_runs,
            output,
        } => {
            match output {
                StatsOutputFormat::Text => {
                    stats_output_text(scenario_name, previous_runs, &db_conn).await?;
                }
                StatsOutputFormat::Json => {
                    stats_output_json(scenario_name, previous_runs, &db_conn).await?;
                }
            }
        }

        Commands::Ui { port } => {
            let port = port.unwrap_or(1337);
            server::start(port, &db_conn).await?
        }
    }

    Ok(())
}
