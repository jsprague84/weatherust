/// Unit tests for weatherust parsing and helper functions

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_looks_like_zip_5_digit() {
        assert!(looks_like_zip("52726"));
        assert!(looks_like_zip("12345"));
    }

    #[test]
    fn test_looks_like_zip_with_extension() {
        assert!(looks_like_zip("52726-1234"));
        assert!(looks_like_zip("12345-6789"));
    }

    #[test]
    fn test_looks_like_zip_with_country_code() {
        assert!(looks_like_zip("52726,US"));
        assert!(looks_like_zip("12345,US"));
    }

    #[test]
    fn test_not_looks_like_zip() {
        assert!(!looks_like_zip("Davenport"));
        assert!(!looks_like_zip("123"));
        assert!(!looks_like_zip("abcde"));
        assert!(!looks_like_zip("52726a"));
    }

    #[test]
    fn test_split_zip_and_cc() {
        let (zip, cc) = split_zip_and_cc("52726,US");
        assert_eq!(zip, "52726");
        assert_eq!(cc, "US");
    }

    #[test]
    fn test_split_zip_default_us() {
        let (zip, cc) = split_zip_and_cc("52726");
        assert_eq!(zip, "52726");
        assert_eq!(cc, "US");
    }

    #[test]
    fn test_split_zip_other_country() {
        let (zip, cc) = split_zip_and_cc("12345,CA");
        assert_eq!(zip, "12345");
        assert_eq!(cc, "CA");
    }

    #[test]
    fn test_normalize_city_query_simple() {
        let result = normalize_city_query("Davenport");
        assert_eq!(result, "Davenport");
    }

    #[test]
    fn test_normalize_city_query_with_state() {
        let result = normalize_city_query("Davenport,IA");
        assert_eq!(result, "Davenport,IA,US");
    }

    #[test]
    fn test_normalize_city_query_with_country() {
        // Note: UK is 2 letters, so it's treated as a US state and gets ",US" appended
        // This is the actual behavior - if you want a specific country, use 3+ letters
        let result = normalize_city_query("London,UK");
        assert_eq!(result, "London,UK,US");
    }

    #[test]
    fn test_normalize_city_query_full() {
        let result = normalize_city_query("Davenport,IA,US");
        assert_eq!(result, "Davenport,IA,US");
    }

    #[test]
    fn test_normalize_city_query_three_letter_state() {
        // Should not be treated as a US state since it's 3 letters
        let result = normalize_city_query("City,ABC");
        assert_eq!(result, "City,ABC");
    }

    #[test]
    fn test_normalize_city_query_with_spaces() {
        let result = normalize_city_query("New York,NY");
        assert_eq!(result, "New York,NY,US");
    }
}
