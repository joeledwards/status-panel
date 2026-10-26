use anyhow::{Result, bail};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize)]
pub struct Dimensions {
    pub columns: usize,
    pub rows: usize,
}

impl Dimensions {
    pub fn new(columns: usize, rows: usize) -> Result<Self> {
        if columns == 0 || rows == 0 {
            bail!("display dimensions must be greater than zero");
        }
        columns.checked_mul(rows).ok_or_else(|| anyhow::anyhow!("display dimensions are too large"))?;
        Ok(Self { columns, rows })
    }

    pub fn len(self) -> usize { self.columns * self.rows }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Frame {
    dimensions: Dimensions,
    cells: Vec<u8>,
}

impl Frame {
    pub fn blank(dimensions: Dimensions) -> Self {
        Self { dimensions, cells: vec![b' '; dimensions.len()] }
    }

    pub fn from_lines<I, S>(dimensions: Dimensions, lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut frame = Self::blank(dimensions);
        for (row, line) in lines.into_iter().take(dimensions.rows).enumerate() {
            frame.write_line(row, line.as_ref());
        }
        frame
    }

    pub fn dimensions(&self) -> Dimensions { self.dimensions }
    pub fn cells(&self) -> &[u8] { &self.cells }

    pub fn row(&self, row: usize) -> &[u8] {
        let start = row * self.dimensions.columns;
        &self.cells[start..start + self.dimensions.columns]
    }

    pub fn write_line(&mut self, row: usize, text: &str) {
        if row >= self.dimensions.rows { return; }
        let start = row * self.dimensions.columns;
        let target = &mut self.cells[start..start + self.dimensions.columns];
        target.fill(b' ');
        for (cell, byte) in target.iter_mut().zip(text.bytes()) {
            *cell = if byte.is_ascii() { byte } else { b'?' };
        }
    }

    pub fn lines(&self) -> impl Iterator<Item = &[u8]> {
        self.cells.chunks(self.dimensions.columns)
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("dimensions", &self.dimensions)
            .field("lines", &self.lines().map(String::from_utf8_lossy).collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_truncated_and_padded() {
        let d = Dimensions::new(4, 2).unwrap();
        let frame = Frame::from_lines(d, ["abcdef", "x"]);
        assert_eq!(frame.cells(), b"abcdx   ");
    }

    #[test]
    fn non_ascii_is_replaced_per_byte() {
        let d = Dimensions::new(3, 1).unwrap();
        assert_eq!(Frame::from_lines(d, ["aé"]).cells(), b"a??");
    }
}
