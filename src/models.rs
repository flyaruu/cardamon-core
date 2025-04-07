use std::{future::Future, pin::Pin};

use crate::{config::Power, data::Data, entities::metrics::Model as Metrics};
use itertools::Itertools;

pub type BoxFuture = Pin<Box<dyn Future<Output = anyhow::Result<Data>> + Send>>;

fn boa_model(a: f64, b: f64, c: f64, d: f64) -> impl Fn(f64) -> f64 {
    move |workload| a * (b * (workload + c)).ln() + d
}

const CORE_COUNT: f64 = 4.0;

pub fn rab_model(metrics: &Vec<&Metrics>, power: &Power, ci_g_wh: f64) -> Data {
    let data = metrics
        .iter()
        .sorted_by(|a, b| b.time_stamp.cmp(&a.time_stamp))
        .tuple_windows()
        .map(|(x, y)| {
            match *power {
                Power::Curve(a, b, c, d) => {
                    let cpu_util = 0.5 * (x.cpu_usage + y.cpu_usage) * 100.0;
                    let delta_t_h = (x.time_stamp - y.time_stamp) as f64 / 3_600_000.0;

                    // boa_model(a, b, c, d)(cpu_util * delta_t_h)
                    boa_model(a, b, c, d)(cpu_util) * delta_t_h
                }

                Power::Tdp(tdp) => {
                    
                    let delta_t_millis = (x.time_stamp - y.time_stamp) as f64;
                    let delta_t_h = delta_t_millis / 3_600_000.0;
                    // taking the midpoint of the two datapoints and dividing by 50 because we're
                    // assuming tdp is at 50% utilization

                    // I don't think cpu_usage is a percentage
                    let avg_cpu = 0.5 * (x.cpu_usage + y.cpu_usage);
                    // println!("Process1:: {} Process2:: {} Delta_t: {} avg_cpu: {}",x.process_name,y.process_name, delta_t_millis,avg_cpu);
                    println!("millis: {} name: {} - {} - avg: {}", delta_t_millis,x.process_name,y.process_name, avg_cpu);
                    avg_cpu / 0.5 * tdp * delta_t_h / CORE_COUNT
                }
            }
        })
        .collect_vec();

    let pow_w = data.iter().fold(0_f64, |acc,x| x + acc);
    println!("Total power: {}", pow_w);
    let co2_g_wh = pow_w * ci_g_wh;

    Data {
        pow: pow_w,
        co2: co2_g_wh,
    }
}
