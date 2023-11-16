#[cfg(test)]
pub mod tests {
    use crate::bencode::debencode;
    use serde_json::json;

    #[test]
    fn test_parse_int() {
        assert_eq!(debencode("i42e".to_owned()), 42);
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(debencode("4:i42e".to_owned()), "i42e");
    }

    #[test]
    fn test_parse_list() {
        assert_eq!(debencode("le".to_owned()), json!([]));
        assert_eq!(debencode("l4:spami42ee".to_owned()), json!(["spam", 42]));
        assert_eq!(
            debencode("l4:spami42el5:hello5:worldee".to_owned()),
            json!(["spam", 42, ["hello", "world"]])
        );
        assert_eq!(debencode("lli4eei5ee".to_owned()), json!([[4], 5]));
    }

    #[test]
    fn test_parse_dict() {
        assert_eq!(
            debencode("d3:bar4:spam3:fooi42ee".to_owned()),
            json!({"bar":"spam", "foo": 42})
        );
        assert_eq!(
            debencode("d3:foo3:bar5:helloi52ee".to_owned()),
            json!({"foo":"bar","hello":52})
        );
    }
}
