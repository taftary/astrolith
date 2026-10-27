//! Trailing window options (#994, #1022 T1): `--build-workers <n>`,
//! `--size <w>x<h>` and `--scale <f>`, in any order after a window run.
//!
//! `--build-workers` sets the ground build threads (the phone-like run,
//! #994). `--size` opens the window at that many physical pixels: the plain
//! window and a `--timing` run take it (a timing run's default is the #819
//! 1600 x 900); the built-in shots set their own sizes, so a plain
//! `--screens` run refuses it. `--scale` overrides the desktop scale factor
//! (a measuring aid for the DPI cases of #1022 R3); without it the window
//! follows the desktop and `--screens` keeps 1.0. Pure: unit-tested below.

/// Largest window side `--size` accepts, in physical pixels.
const MAX_SIDE: u32 = 8192;

/// Scale factors `--scale` accepts.
const SCALES: std::ops::RangeInclusive<f32> = 0.5..=4.0;

/// The trailing options of a window run.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct WindowOptions {
    /// Ground build threads (`--build-workers`).
    pub workers: Option<usize>,
    /// Window size in physical pixels (`--size`).
    pub size: Option<(u32, u32)>,
    /// Scale factor override (`--scale`).
    pub scale: Option<f32>,
}

/// Parses `<w>x<h>`: both sides from 1 to [`MAX_SIDE`].
fn parse_size(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.split_once('x')?;
    let side = |s: &str| s.parse::<u32>().ok().filter(|n| (1..=MAX_SIDE).contains(n));
    Some((side(w)?, side(h)?))
}

/// Parses a scale factor within [`SCALES`].
fn parse_scale(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|f| SCALES.contains(f))
}

/// Splits the trailing options off `args`: the rest plus the options.
/// `None` for a wrong or repeated value.
pub(crate) fn split_options(args: &[String]) -> Option<(&[String], WindowOptions)> {
    let mut rest = args;
    let mut options = WindowOptions::default();
    while let [head @ .., flag, value] = rest {
        match flag.as_str() {
            "--build-workers" if options.workers.is_none() => {
                options.workers = Some(value.parse::<usize>().ok().filter(|n| *n > 0)?);
            }
            "--size" if options.size.is_none() => options.size = Some(parse_size(value)?),
            "--scale" if options.scale.is_none() => options.scale = Some(parse_scale(value)?),
            "--build-workers" | "--size" | "--scale" => return None,
            _ => break,
        }
        rest = head;
    }
    Some((rest, options))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_string()).collect()
    }

    #[test]
    fn options_split_off_in_any_order() {
        let all = args(&[
            "--screens",
            "out",
            "--size",
            "1600x900",
            "--scale",
            "1.5",
            "--build-workers",
            "2",
        ]);
        let (rest, options) = split_options(&all).expect("valid");
        assert_eq!(rest, &all[..2]);
        assert_eq!(
            options,
            WindowOptions {
                workers: Some(2),
                size: Some((1600, 900)),
                scale: Some(1.5),
            }
        );
        let none = args(&["--verify"]);
        assert_eq!(
            split_options(&none),
            Some((&none[..], WindowOptions::default()))
        );
    }

    #[test]
    fn wrong_or_repeated_options_are_refused() {
        for wrong in [
            &["--size", "1600"][..],
            &["--size", "0x900"],
            &["--size", "9000x900"],
            &["--scale", "0.1"],
            &["--scale", "big"],
            &["--build-workers", "0"],
            &["--size", "800x600", "--size", "800x600"],
        ] {
            assert_eq!(split_options(&args(wrong)), None, "{wrong:?}");
        }
    }
}
