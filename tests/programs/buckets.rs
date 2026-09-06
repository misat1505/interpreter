use std::io::BufReader;

use crate::common::assert_same_output;

#[test]
fn buckets() {
    let text = BufReader::new(
        r##"
struct Bucket {
    i64[] items,
    str label
};

fn make_bucket(i64 n): Bucket {
    let items: i64[] = [];
    let i: i64 = 0;

    while (i < n) {
        vector_push(&items, i * i);
        i = i + 1;
    }

    return Bucket { items: items, label: "generated" };
}

fn sum_items(Bucket[] b): i64 {
    let total: i64 = 0;
    let i: i64 = 0;

    while (i < vector_size(&b)) {
        let bucket: Bucket = b[i];
        let j: i64 = 0;

        while (j < vector_size(&bucket.items)) {
            total = total + bucket.items[j];
            j = j + 1;
        }

        i = i + 1;
    }

    return total;
}

fn main(): void {
    let buckets: Bucket[] = [];

    let k: i64 = 0;
    while (k < 5) {
        let b: Bucket = make_bucket(k);
        vector_push(&buckets, b);
        k = k + 1;
    }

    let result: i64 = sum_items(buckets);
    println(result as str);
}

main();
    "##
        .as_bytes(),
    );

    assert_same_output(text, "20\n");
}
