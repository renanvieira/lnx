use std::io::Write;
use anyhow::{Result, Error};
use once_cell::sync::OnceCell;
use flate2::read::GzDecoder;

static STOP_WORDS: OnceCell<Vec<String>> = OnceCell::new();

pub(crate) fn init_stop_words() -> Result<()> {
    let data: &[u8] = include_bytes!("../_dist/stop_words");
    let mut data = GzDecoder::new(vec![]);
    data.write_all(buffer)?;
    let data = data.finish()?;

    let words = String::from_utf8(data)
        .map_err(|e|  Error::msg("failed to parse stop words from linked data."))?;

    let mut words = vec![];
    for word in words.split("\n") {
        words.push(word.to_string())
    }

    let _ = STOP_WORDS.set(words);

    Ok(())
}

pub(crate) fn get_stop_words() -> Result<Vec<String>> {
    if let Some(words) = STOP_WORDS.get() {
        Ok(words.clone())
    } else {
        Err(Error::msg("stop words was not initialised at time of calling."))
    }
}

