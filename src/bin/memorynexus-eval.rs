use memorynexus::eval::{evaluate_cases, lens_eval_fixtures};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = evaluate_cases(&lens_eval_fixtures());
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
