use std::io::BufReader;

use crate::common::assert_same_output;

#[test]
fn split_path() {
    let text = BufReader::new(
        r##"
fn split_by_delim(str text, char delim): str[] {
    let segments: str[] = [];
    let word = "";

    for (let i = 0; i < str_len(text); i += 1) {
        let c = text[i];
        if (c == delim) {
            vector_push(&segments, word);
            word = "";
        } else {
            word += c;
        }
    }

    vector_push(&segments, word);

    return segments;
}

let segments = split_by_delim("Users/user1/Desktop/foo/bar/baz", '/');
println(vector_stringify(segments));
    "##
        .as_bytes(),
    );

    assert_same_output(text, "[\"Users\", \"user1\", \"Desktop\", \"foo\", \"bar\", \"baz\"]\n");
}
