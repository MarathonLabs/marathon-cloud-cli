use serde_with::DurationSecondsWithFrac;
use std::{fmt::Display, time::Duration};

use serde::Serialize;
use serde_with::serde_as;

#[derive(Serialize)]
pub struct TestRunStarted {
    pub id: String,
}

impl Display for TestRunStarted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Test run {} started", self.id))
    }
}

#[serde_as]
#[derive(Serialize)]
pub struct TestRunFinished {
    pub id: String,
    pub report: String,
    pub state: String,
    pub passed: Option<u32>,
    pub failed: Option<u32>,
    pub ignored: Option<u32>,
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    pub billable_time: Duration,
}

impl Display for TestRunFinished {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.state.as_ref() {
            "passed" => f.write_str("Marathon Cloud execution finished\n")?,
            "failure" => f.write_str("Marathon Cloud execution finished with failures\n")?,
            _ => f.write_str("Marathon cloud execution crashed\n")?,
        };
        f.write_fmt(format_args!("\tstate: {}\n", self.state))?;

        f.write_fmt(format_args!("\treport: {}\n", self.report))?;
        f.write_fmt(format_args!(
            "\tpassed: {}\n",
            self.passed
                .map(|x| x.to_string())
                .unwrap_or("missing".to_owned()),
        ))?;
        f.write_fmt(format_args!(
            "\tfailed: {}\n",
            self.failed
                .map(|x| x.to_string())
                .unwrap_or("missing".to_owned()),
        ))?;
        f.write_fmt(format_args!(
            "\tignored: {}\n",
            self.ignored
                .map(|x| x.to_string())
                .unwrap_or("missing".to_owned()),
        ))?;

        let s = self.billable_time.as_secs();
        let ms = self.billable_time.subsec_millis();
        let (h, s) = (s / 3600, s % 3600);
        let (m, s) = (s / 60, s % 60);
        let formatted_billable_time = format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms);

        f.write_fmt(format_args!(
            "\tbillable time: {}\n",
            formatted_billable_time
        ))?;
        Ok(())
    }
}
