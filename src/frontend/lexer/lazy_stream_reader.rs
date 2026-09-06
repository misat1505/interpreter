use std::error::Error;
use std::io::BufRead;

use crate::common::position::Position;

pub const STX: char = '\u{2}';
pub const ETX: char = '\u{3}';

pub trait ILazyStreamReader {
    fn current(&self) -> &char;
    fn next(&mut self) -> Result<&char, Box<dyn Error>>;
    fn position(&self) -> Position;
}

pub struct LazyStreamReader<R: BufRead> {
    src: R,
    current_line: String,
    current_char: char,
    newline: Option<Vec<u8>>,
    current_position: Position,
    filename: Option<&'static str>,
}

impl<R: BufRead> ILazyStreamReader for LazyStreamReader<R> {
    fn current(&self) -> &char {
        &self.current_char
    }

    fn next(&mut self) -> Result<&char, Box<dyn Error>> {
        let new_char = self.read_char()?;
        self.update_position(self.current_char);
        self.current_char = new_char;
        Ok(&self.current_char)
    }

    fn position(&self) -> Position {
        self.current_position
    }
}

impl<R: BufRead> LazyStreamReader<R> {
    pub fn new(src: R, filename: Option<&'static str>) -> LazyStreamReader<R> {
        LazyStreamReader {
            src,
            current_line: String::new(),
            current_char: STX,
            newline: None,
            current_position: Position::new(0, 0, 0, filename),
            filename,
        }
    }

    fn read_char(&mut self) -> Result<char, Box<dyn Error>> {
        let new_char = match self.try_handle_newline()? {
            None => self.process_char()?,
            Some(c) => c,
        };

        Ok(new_char)
    }

    fn try_handle_newline(&mut self) -> Result<Option<char>, Box<dyn Error>> {
        let buffer = self.src.fill_buf()?;

        if let Some(&first_char) = buffer.first() {
            if let Some(&second_char) = buffer.get(1) {
                if first_char == b'\r' {
                    let mut newline_sequence = vec![first_char];
                    self.src.consume(1);
                    if second_char == b'\n' {
                        newline_sequence.push(second_char);
                        self.src.consume(1);
                    }
                    self.newline = Some(newline_sequence.clone());
                    return Ok(Some('\n'));
                } else if first_char == b'\n' {
                    self.src.consume(1);
                    self.newline = Some(vec![first_char]);
                    return Ok(Some('\n'));
                }
            }
        }

        Ok(None)
    }

    fn process_char(&mut self) -> Result<char, Box<dyn Error>> {
        let buffer = self.src.fill_buf()?;

        if buffer.is_empty() {
            return Ok(ETX);
        }

        let first_byte = *buffer.first().unwrap();
        let char = first_byte as char;

        self.src.consume(1);

        Ok(char)
    }

    fn update_position(&mut self, read_character: char) {
        match read_character {
            STX => {
                self.current_position = Position::new(1, 1, 0, self.filename);
            }
            ETX => {}
            '\n' => {
                self.current_position.offset += self.newline.as_ref().unwrap().len();
                self.current_position.line += 1;
                self.current_position.column = 1;
                self.current_line = String::new();
            }
            char => {
                self.current_position.offset += self.current_char.len_utf8();
                self.current_position.column += 1;
                self.current_line.push(char);
            }
        };
    }

    #[allow(dead_code)]
    pub fn error_code_snippet(&mut self) -> String {
        let mut buffer = String::new();
        let _ = self.src.read_line(&mut buffer);

        let spaces = " ".repeat((self.position().column - 1) as usize);
        let caret_string = format!("{}^", spaces);

        format!("\nAt line:\n{}{}{}{}", self.current_line, self.current_char, buffer, caret_string)
    }
}
