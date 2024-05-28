use console::style;

pub trait Formatter {
    fn stage(&self, message: &str);
    fn message(&self, message: &str);
}

pub struct StandardFormatter {
    stage_count: u32,
    index: u32,
}

impl StandardFormatter {
    pub fn new(stage_count: u32) -> Self {
        Self {
            stage_count,
            index: 1,
        }
    }
}

impl Formatter for StandardFormatter {
    fn stage(&self, message: &str) {
        let stage_prefix = style(format!("[{}/{}]", self.index, self.stage_count))
            .bold()
            .dim();
        let message = format!("{} {}", stage_prefix, message);
        println!("{}", &message);
    }

    fn message(&self, message: &str) {
        println!("{}", &message);
    }
}
