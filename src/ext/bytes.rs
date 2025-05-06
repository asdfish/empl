use std::str;

pub trait BytesExt: AsRef<[u8]> {
    fn chars<'a>(&'a self) -> Chars<'a> {
        Chars(self.as_ref())
    }
}
impl<T> BytesExt for T where T: AsRef<[u8]> {}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Chars<'a>(&'a [u8]);
impl<'a> Iterator for Chars<'a> {
    type Item = Option<char>;

    fn next(&mut self) -> Option<Option<char>> {
        if self.0.is_empty() {
            None
        } else {
            (1..=4)
                .flat_map(|i| self.0.get(..i))
                .flat_map(str::from_utf8)
                .flat_map(|ch| ch.chars().next())
                .next()
                .inspect(|ch| {
                    self.0 = self.0.get(..ch.len_utf8()).unwrap_or_default();
                })
                .map(Some)
        }
    }
}
