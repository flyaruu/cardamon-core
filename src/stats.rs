use chrono::{TimeZone, Utc};
use colored::Colorize;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use term_table::table_cell::TableCell;
use term_table::{row, rows, Table, TableStyle};
use term_table::row::Row;

use crate::data::{dataset::LiveDataFilter, dataset_builder::DatasetBuilder};
use crate::models::rab_model;


pub async fn stats_output_text(scenario_name: Option<String>,previous_runs: Option<u64>, db_conn: &DatabaseConnection)->anyhow::Result<()> {
            // build dataset
            let dataset_builder = DatasetBuilder::new();
            let dataset_rows = match scenario_name {
                Some(scenario_name) => dataset_builder.scenario(&scenario_name).all(),
                None => dataset_builder.scenarios_all().all(),
            };
            let dataset_cols = match previous_runs {
                Some(n) => dataset_rows.last_n_runs(n).all(),
                None => dataset_rows.runs_all().all(),
            };
            let dataset = dataset_cols.build(&db_conn).await?;

            println!("\n{}", " Cardamon Stats \n".reversed().green());
            if dataset.is_empty() {
                println!("\nno data found!");
            }

            for scenario_dataset in dataset.by_scenario(LiveDataFilter::IncludeLive) {
                println!(
                    "Scenario {}:",
                    scenario_dataset.scenario_name().to_string().green()
                );

                let mut table = Table::builder()
                    .rows(rows![row![
                        TableCell::builder("Datetime (Utc)".bold()).build(),
                        TableCell::builder("Region".bold()).build(),
                        TableCell::builder("Duration (s)".bold()).build(),
                        TableCell::builder("Power (Wh)".bold()).build(),
                        TableCell::builder("CI (gWh)".bold()).build(),
                        TableCell::builder("CO2 (g)".bold()).build()
                    ]])
                    .style(TableStyle::rounded())
                    .build();

                for run_dataset in scenario_dataset.by_run() {
                    let run_data = run_dataset.apply_model(&db_conn, &rab_model).await?;
                    let run_region = run_data.region;
                    let run_ci = run_data.ci;
                    let run_start_time = Utc.timestamp_opt(run_data.start_time / 1000, 0).unwrap();
                    let run_duration = (run_data.stop_time - run_data.start_time) as f64 / 1000.0;
                    let _per_min_factor = 60.0 / run_duration;

                    table.add_row(row![
                        TableCell::new(run_start_time.format("%d/%m/%y %H:%M")),
                        TableCell::new(run_region.unwrap_or_default()),
                        TableCell::new(format!("{:.3}s", run_duration)),
                        TableCell::new(format!("{:.4}Wh", run_data.data.pow)),
                        TableCell::new(format!("{:.4}gWh", run_ci)),
                        TableCell::new(format!("{:.4}g", run_data.data.co2)),
                    ]);
                    // points.push((run, run_data.data.pow as f32));
                    // run += 1.0;
                }
                println!("{}", table.render());

                // let x_max = points.len() as f32;
                // let y_data = points.iter().map(|(_, y)| *y);
                // let y_min = y_data.clone().reduce(f32::min).unwrap_or(0.0);
                // let y_max = y_data.clone().reduce(f32::max).unwrap_or(0.0);
                //
                // Chart::new_with_y_range(128, 64, 0.0, x_max, y_min, y_max)
                //     .x_axis_style(textplots::LineStyle::Solid)
                //     .y_tick_display(TickDisplay::Sparse)
                //     .lineplot(&Shape::Lines(&points))
                //     .nice();
            }
        Ok(())

}

#[derive(Debug, Clone, Serialize)]
struct ScenarioOutput {
    name: String,
    region: String,
    duration: f64,
    power: f64,
    ci: f64,
    co2: f64,
}

#[derive(Debug, Clone, Serialize)]
struct RunOutput {
    name: String,
    outputs: Vec<ScenarioOutput>,
}

pub async fn stats_output_json(scenario_name: Option<String>,previous_runs: Option<u64>, db_conn: &DatabaseConnection)->anyhow::Result<()> {
            // build dataset
            let dataset_builder = DatasetBuilder::new();
            let dataset_rows = match scenario_name {
                Some(scenario_name) => dataset_builder.scenario(&scenario_name).all(),
                None => dataset_builder.scenarios_all().all(),
            };
            let dataset_cols = match previous_runs {
                Some(n) => dataset_rows.last_n_runs(n).all(),
                None => dataset_rows.runs_all().all(),
            };
            let dataset = dataset_cols.build(&db_conn).await?;
            let mut run_outputs: Vec<RunOutput> = vec![];
            for scenario_dataset in dataset.by_scenario(LiveDataFilter::IncludeLive) {
                let mut scenario_outputs: Vec<ScenarioOutput> = vec![];
                let scenario_name = scenario_dataset.scenario_name().to_string();
                for run_dataset in scenario_dataset.by_run() {
                    let run_data = run_dataset.apply_model(&db_conn, &rab_model).await?;
                    let run_region = run_data.region;
                    let run_ci = run_data.ci;
                    let _run_start_time = Utc.timestamp_opt(run_data.start_time / 1000, 0).unwrap();
                    let run_duration = (run_data.stop_time - run_data.start_time) as f64 / 1000.0;
                    let _per_min_factor = 60.0 / run_duration;
                    let stats_output = ScenarioOutput {
                        name: scenario_name.clone(),
                        region: run_region.unwrap_or_default(),
                        duration: run_duration,
                        power: run_data.data.pow,
                        ci: run_ci,
                        co2: run_data.data.co2,
                    };
                    scenario_outputs.push(stats_output);
                }
                let scenario_output = RunOutput {
                    name: scenario_name,
                    outputs: scenario_outputs,
                };
                run_outputs.push(scenario_output);
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&run_outputs)?
            );

        Ok(())

}
