use crossterm::style::Colors;

/// Extension trait for [Colors]
pub trait ColorsExt {
    /// Combine two [Colors] by replacing the color in `self` it they exist in `r`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::ext::colors::ColorsExt;
    /// # use crossterm::style::{Color, Colors};
    /// fn test_join([mut init, join_with, output]: [Colors; 3]) {
    ///     init.join(&join_with);
    ///     assert_eq!(init, output);
    /// }
    ///
    /// test_join([
    ///     Colors {
    ///         foreground: None,
    ///         background: None,
    ///     },
    ///     Colors::new(Color::Red, Color::Red),
    ///     Colors::new(Color::Red, Color::Red),
    /// ]);
    /// test_join([
    ///     Colors::new(Color::Blue, Color::Blue),
    ///     Colors::new(Color::Red, Color::Red),
    ///     Colors::new(Color::Red, Color::Red),
    /// ]);
    /// test_join([
    ///     Colors::new(Color::Blue, Color::Blue),
    ///     Colors {
    ///         foreground: None,
    ///         background: None,
    ///     },
    ///     Colors::new(Color::Blue, Color::Blue),
    /// ])
    /// ```
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
