use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, Cursor, Read};
use std::sync::Arc;
use std::thread;

#[derive(Clone, Copy, PartialEq, Debug)]
enum CountType {
    ByteCount,
    CharCount,
    WordCount,
    LineCount,
    AllCount,
}

#[derive(Clone)]
pub struct Config {
    count_type: CountType,
    file_path: Option<String>,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, &'static str> {
        // Case: only std input
        if args.len() == 1 {
            return Ok(Config {
                count_type: CountType::AllCount,
                file_path: None,
            });
        }

        // Case: Only file path is provided
        if args.len() == 2 && !args[1].starts_with('-') {
            return Ok(Config {
                count_type: CountType::AllCount,
                file_path: Some(args[1].clone()),
            });
        }

        // Case: Flag is provided
        if args.len() >= 3 && args[1].starts_with('-') {
            if let Some(count_type) = Self::_parse_type(&args[1]) {
                return Ok(Config {
                    count_type,
                    file_path: Some(args[2].clone()),
                });
            } else {
                return Err("Invalid flag. Use 'c' for byte count, 'l' for line count, 'w' for word count, or 'm' for character count.");
            }
        }

        // Case: Only a flag is provided without a file path
        if args.len() == 2 && args[1].starts_with('-') {
            if let Some(count_type) = Self::_parse_type(&args[1]) {
                return Ok(Config {
                    count_type,
                    file_path: None,
                });
            } else {
                return Err("Invalid flag. Use 'c' for byte count, 'l' for line count, 'w' for word count, or 'm' for character count.");
            }
        }

        Err("Incorrect usage. Usage: <program> <flag> <file_path>")
    }

    fn _parse_type(arg: &str) -> Option<CountType> {
        match arg.chars().last()? {
            'c' => Some(CountType::ByteCount),
            'l' => Some(CountType::LineCount),
            'w' => Some(CountType::WordCount),
            'm' => Some(CountType::CharCount),
            _ => None, // Invalid flag case
        }
    }

    fn get_count_type(&self) -> CountType {
        self.count_type
    }
    fn get_file_path(&self) -> Option<String> {
        self.file_path.clone()
    }
}

pub struct Counter {
    count_type: CountType,
    file_path: Option<String>,
}

impl Counter {
    pub fn count(self) -> Result<(), Box<dyn Error>> {
        let filename = match &self.file_path {
            Some(file_path) => file_path,
            None => &String::from(""),
        };
        match self.count_type {
            CountType::AllCount => {
                // Concurrently calculate bytes, lines, and words
                let (byte_count, line_count, word_count) = self.count_all()?;
                println!(
                    "{}\t{}\t{} {}",
                    line_count, word_count, byte_count, filename
                )
            }
            CountType::ByteCount => {
                let count = self.count_bytes()?;
                println!("{} {}", count, filename);
            }
            CountType::LineCount => {
                let count = self.count_lines()?;
                println!("{} {}", count, filename);
            }
            CountType::WordCount => {
                let count = self.count_words()?;
                println!("{} {}", count, filename);
            }
            CountType::CharCount => {
                let count = self.count_chars()?;
                println!("{} {}", count, filename);
            }
        }
        Ok(())
    }

    pub fn count_all(&self) -> Result<(usize, usize, usize), io::Error> {
        // Read entire input once to ensure safe concurrent access
        let input_data = Arc::new(self.read_input()?);
        Self::count_all_from_input(input_data)
    }

    pub fn count_bytes(&self) -> Result<usize, io::Error> {
        let input_data = self.read_input()?;
        Self::count_bytes_from_reader(Cursor::new(input_data.as_str()))
    }

    pub fn count_lines(&self) -> Result<usize, io::Error> {
        let input_data = self.read_input()?;
        Self::count_lines_from_reader(Cursor::new(input_data.as_str()))
    }

    pub fn count_words(&self) -> Result<usize, io::Error> {
        let input_data = self.read_input()?;
        Self::count_words_from_reader(Cursor::new(input_data.as_str()))
    }

    pub fn count_chars(&self) -> Result<usize, io::Error> {
        let input_data = self.read_input()?;
        Self::count_chars_from_reader(Cursor::new(input_data.as_str()))
    }

    fn read_input(&self) -> Result<String, io::Error> {
        let mut buffer = String::new();
        if let Some(ref path) = self.file_path {
            let mut file = File::open(path)?;
            file.read_to_string(&mut buffer)?;
        } else {
            io::stdin().read_to_string(&mut buffer)?;
        }
        Ok(buffer)
    }

    fn count_bytes_from_reader<R: BufRead>(mut reader: R) -> Result<usize, io::Error> {
        let mut total_bytes = 0;
        let mut buffer = [0; 1024];
        while let Ok(bytes_read) = reader.read(&mut buffer) {
            if bytes_read == 0 {
                break;
            }
            total_bytes += bytes_read;
        }
        Ok(total_bytes)
    }

    fn count_lines_from_reader<R: BufRead>(reader: R) -> Result<usize, io::Error> {
        Ok(reader.lines().count())
    }

    fn count_words_from_reader<R: BufRead>(reader: R) -> Result<usize, io::Error> {
        let mut count = 0;
        for line in reader.lines() {
            count += line?.split_whitespace().count();
        }
        Ok(count)
    }

    fn count_chars_from_reader<R: BufRead>(mut reader: R) -> Result<usize, io::Error> {
        let mut total_chars = 0;
        let mut buffer = String::new();

        while reader.read_to_string(&mut buffer)? > 0 {
            total_chars += buffer.chars().count();
            buffer.clear(); // Clear the buffer for the next chunk of data.
        }

        Ok(total_chars)
    }

    fn count_all_from_input(input_data: Arc<String>) -> Result<(usize, usize, usize), io::Error> {
        let byte_handle = {
            let input_data = Arc::clone(&input_data);
            thread::spawn(move || Self::count_bytes_from_reader(Cursor::new(input_data.as_str())))
        };

        let line_handle = {
            let input_data = Arc::clone(&input_data);
            thread::spawn(move || Self::count_lines_from_reader(Cursor::new(input_data.as_str())))
        };

        let word_handle = {
            let input_data = Arc::clone(&input_data);
            thread::spawn(move || Self::count_words_from_reader(Cursor::new(input_data.as_str())))
        };

        let byte_count = byte_handle.join().unwrap()?;
        let line_count = line_handle.join().unwrap()?;
        let word_count = word_handle.join().unwrap()?;

        Ok((byte_count, line_count, word_count))
    }
}

impl From<Config> for Counter {
    fn from(config: Config) -> Self {
        Counter {
            count_type: config.get_count_type(),
            file_path: config.get_file_path(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_count_bytes() {
        let input_data = "Hello, world!";
        let cursor = Cursor::new(input_data);
        let byte_count = Counter::count_bytes_from_reader(cursor).unwrap();
        assert_eq!(byte_count, input_data.len());
    }

    #[test]
    fn test_count_chars() {
        let input_data = "Hello, 🌍!";
        let cursor = Cursor::new(input_data);
        let char_count = Counter::count_chars_from_reader(cursor).unwrap();
        assert_eq!(char_count, input_data.chars().count());
    }

    #[test]
    fn test_count_words() {
        let input_data = "Hello world, how are you?";
        let cursor = Cursor::new(input_data);
        let word_count = Counter::count_words_from_reader(cursor).unwrap();
        assert_eq!(word_count, 5); // "Hello", "world,", "how", "are", "you?"
    }

    #[test]
    fn test_count_lines() {
        let input_data = "Line one\nLine two\nLine three";
        let cursor = Cursor::new(input_data);
        let line_count = Counter::count_lines_from_reader(cursor).unwrap();
        assert_eq!(line_count, 3);
    }

    #[test]
    fn test_count_all() {
        let input_data = String::from("Hello, world!\nRust is fun.");

        // Use Cursor to simulate stdin with `input_data`
        let mock_stdin = Arc::new(input_data.clone());

        // Pass `Some(mock_stdin)` as the reader to `count_all`
        let (byte_count, line_count, word_count) =
            Counter::count_all_from_input(mock_stdin).unwrap();

        // Expected counts based on input
        let expected_bytes = input_data.len();
        let expected_lines = 2;
        let expected_words = 5;

        assert_eq!(byte_count, expected_bytes);
        assert_eq!(line_count, expected_lines);
        assert_eq!(word_count, expected_words);
    }

    #[test]
    fn test_config_build_with_flag_and_file_path() {
        let args = vec!["gfwc".to_string(), "-l".to_string(), "test.txt".to_string()];
        let config = Config::build(&args).unwrap();
        assert_eq!(config.count_type, CountType::LineCount);
        assert_eq!(config.file_path, Some("test.txt".to_string()));
    }

    #[test]
    fn test_config_build_with_only_file_path() {
        let args = vec!["gfwc".to_string(), "test.txt".to_string()];
        let config = Config::build(&args).unwrap();
        assert_eq!(config.count_type, CountType::AllCount);
        assert_eq!(config.file_path, Some("test.txt".to_string()));
    }

    #[test]
    fn test_config_invalid_flag() {
        let args = vec!["gfwc".to_string(), "-z".to_string()];
        let config = Config::build(&args);
        assert!(config.is_err());
    }

    #[test]
    fn test_config_no_flag_defaults_to_all_count() {
        let args = vec!["gfwc".to_string(), "text.txt".to_string()];
        let config = Config::build(&args).unwrap();
        assert_eq!(config.count_type, CountType::AllCount);
        assert_eq!(config.file_path, Some("text.txt".to_string()));
    }

    #[test]
    fn test_config_valid_flag() {
        let args = vec!["gfwc".to_string(), "-w".to_string(), "text.txt".to_string()];
        let config = Config::build(&args).unwrap();
        assert_eq!(config.count_type, CountType::WordCount);
        assert_eq!(config.file_path, Some("text.txt".to_string()));
    }

    #[test]
    fn test_config_only_flag_no_file_path() {
        let args = vec!["gfwc".to_string(), "-w".to_string()];
        let config = Config::build(&args).unwrap();
        assert_eq!(config.count_type, CountType::WordCount);
        assert!(config.file_path.is_none());
    }
}
