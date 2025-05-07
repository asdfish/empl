use crossterm::style::Colors;

pub trait ColorsExt {
    fn join(&mut self, r: &Colors);
}
impl ColorsExt for Colors {
    fn join(&mut self, r: &Colors) {
        if let Some(foreground) = r.foreground {
            self.foreground = Some(foreground);
        }
        if let Some(background) = r.background {
            self.background = Some(background);
        }
    }
}
