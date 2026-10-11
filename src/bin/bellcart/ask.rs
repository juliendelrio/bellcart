//! Questions to the user on the terminal.

use std::io::BufRead;

use anyhow::Result;

/// Reads one answer. Only an explicit yes counts: an empty line, the end
/// of the input or anything else means no.
pub fn confirmed(answers: &mut impl BufRead) -> Result<bool> {
    let mut line = String::new();
    answers.read_line(&mut line)?;
    let answer = line.trim().to_lowercase();
    Ok(["y", "yes", "o", "oui"].contains(&answer.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_explicit_yes_confirms() {
        for (answer, expected) in [
            ("y\n", true),
            ("Yes\n", true),
            (" o \n", true),
            ("OUI\r\n", true),
            ("\n", false),
            ("", false),
            ("n\n", false),
            ("yes please\n", false),
        ] {
            let mut input = answer.as_bytes();
            assert_eq!(confirmed(&mut input).unwrap(), expected, "{answer:?}");
        }
    }
}
