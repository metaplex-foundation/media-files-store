
pub fn keccak256_hash_bs58str(s: &str) -> String {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::default();
    hasher.update(s.as_bytes());
    bs58::encode(hasher.finalize().as_slice()).into_string()
}

pub trait StrUtil {
    fn trim_right_slash(& self) -> & str;
}

impl StrUtil for &str {
    fn trim_right_slash(& self) -> & str {
        let slashes_cnt = self.chars().into_iter().rev().take_while(|c| *c == '/').count();
        &self[0 .. self.len() - ('/'.len_utf8() * slashes_cnt)]
    }
}

#[test]
fn test_trim_right_slash() {
    assert_eq!("/aaa/bbb/".trim_right_slash(), "/aaa/bbb");
    assert_eq!("/aaa/bbb///".trim_right_slash(), "/aaa/bbb");
    assert_eq!("status_spark_1.2.0/".to_string().as_str().trim_right_slash(), "status_spark_1.2.0");
}