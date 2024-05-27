use std::fmt::Display;

use serde::Serialize;

#[derive(Serialize)]
pub struct TestRunStarted {
    pub id: String,
}

impl Display for TestRunStarted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Test run {} started", self.id))
    }
}

#[derive(Serialize)]
pub struct TestRunFinished {
    pub id: String,
    pub report: String,
    pub state: String,
    pub passed: Option<u32>,
    pub failed: Option<u32>,
    pub ignored: Option<u32>,
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
        Ok(())
    }
}
