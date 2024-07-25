use lazy_regex::regex_is_match;

pub fn validate_email(email: &str) -> bool {
    regex_is_match!(r"(?i)^[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}$", email)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
    type Error = Box<dyn std::error::Error>;
    type Result<T> = core::result::Result<T, Error>; // For tests.

    use super::*;

    #[test]
    fn test_validate_email_ok() -> Result<()> {
        assert_eq!(true, validate_email("popo@momo.com"));
        Ok(())
    }
    #[test]
    fn test_validate_email_false() -> Result<()> {
        assert_eq!(false, validate_email("popom"));
        Ok(())
    }
}

// endregion: --- Tests
