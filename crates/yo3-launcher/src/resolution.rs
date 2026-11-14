//! The connection DLL's higher resolution (`tools/mod/resolution.h`): the game keeps its 1024x768 frame and the DLL
//! draws it larger, scaled to fit and centred. The launcher hands the choice to the game as `YO3_RESOLUTION`; the DLL
//! applies the same rules (`tools/mod/resolution-fit.h`) and refuses to start the game on a value it doesn't accept.
use std::fmt;
use std::str::FromStr;

/// The game's environment variable the DLL reads.
pub const ENV: &str = "YO3_RESOLUTION";
/// Each side of a requested size, in pixels.
pub const MIN: u16 = 320;
pub const MAX: u16 = 8192;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Resolution {
    /// The game's own 1024x768 window: nothing is installed.
    #[default]
    Off,
    /// The largest frame the screen holds (the work area for a window, the whole screen for the game's Full mode).
    Auto,
    /// A window of this client size; the game's Full mode always uses the whole screen.
    Size(u16, u16),
}

impl FromStr for Resolution {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, String> {
        let invalid = || format!("{text:?} is not a resolution: use WIDTHxHEIGHT ({MIN} to {MAX} each), auto or off");
        if text.is_empty() || text == "0" || text.eq_ignore_ascii_case("off") || text.eq_ignore_ascii_case("original") {
            return Ok(Self::Off);
        }
        if text.eq_ignore_ascii_case("auto") { return Ok(Self::Auto); }
        let (w, h) = text.split_once(['x', 'X']).ok_or_else(invalid)?;
        let side = |s: &str| (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse::<u16>().ok()).flatten()
            .filter(|n| (MIN..=MAX).contains(n));
        Ok(Self::Size(side(w).ok_or_else(invalid)?, side(h).ok_or_else(invalid)?))
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { Self::Off => f.write_str("off"), Self::Auto => f.write_str("auto"), Self::Size(w, h) => write!(f, "{w}x{h}") }
    }
}

#[cfg(test)]
mod tests {
    use super::Resolution::{self, *};
    #[test]
    fn values_match_the_dll() {
        // The same cases as tools/mod/resolution-test.c.
        for (text, expected) in [("", Off), ("0", Off), ("Off", Off), ("original", Off), ("auto", Auto), ("1440x1080", Size(1440, 1080)),
            ("1920X1080", Size(1920, 1080)), ("320x320", Size(320, 320)), ("8192x8192", Size(8192, 8192))] {
            assert_eq!(text.parse::<Resolution>(), Ok(expected), "{text}");
        }
        for text in ["1440", "1440x", "x1080", "1440x1080x", "-1440x1080", "1440x+1080", " 1440x1080", "100x100", "9000x1080",
            "1440*1080", "banana", "1440x1080 ", "99999999999x1080"] {
            assert!(text.parse::<Resolution>().is_err(), "{text}");
        }
        for value in [Off, Auto, Size(1600, 1200)] { assert_eq!(value.to_string().parse::<Resolution>(), Ok(value)); }
    }
}
