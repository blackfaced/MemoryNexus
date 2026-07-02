use memorynexus::eval::{
    evaluate_cases, evaluate_dictation_bench_recurring_errors, lens_eval_fixtures,
    load_dictation_bench_fixtures,
};
use serde_json::json;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());

    match mode.as_str() {
        "all" => {
            let report = json!({
                "lens_eval": evaluate_cases(&lens_eval_fixtures()),
                "dictation_bench_recurring_errors": dictation_bench_recurring_error_report()?,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "lens" => {
            let report = evaluate_cases(&lens_eval_fixtures());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "dictation-bench-recurring-errors" => {
            let report = dictation_bench_recurring_error_report()?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        other => {
            return Err(format!(
                "unsupported eval mode {other}; use all, lens, or dictation-bench-recurring-errors"
            )
            .into());
        }
    }

    Ok(())
}

fn dictation_bench_recurring_error_report(
) -> Result<memorynexus::eval::DictationBenchRecurringErrorReport, Box<dyn std::error::Error>> {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("dictation_bench");
    let fixtures = load_dictation_bench_fixtures(&fixture_dir)?;
    Ok(evaluate_dictation_bench_recurring_errors(&fixtures))
}
