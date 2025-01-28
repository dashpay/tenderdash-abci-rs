//! Port of Merkle tree implementation in Tenderdash.
//! See https://github.com/dashpay/tenderdash/blob/feat/statesync-integration/crypto/merkle/tree.go
//! for original implementation.

const LEAF_PREFIX: u8 = 0;
const INNER_PREFIX: u8 = 1;

/// Calculate the Merkle root hash of a list of items.
pub(crate) fn merkle_hash<T: AsRef<[u8]>>(items: &[T]) -> [u8; 32] {
    match items.len() {
        0 => empty_hash(),
        1 => leaf(items[0].as_ref()),
        _ => {
            let k = get_split_point(items.len() as i64) as usize;
            let left = merkle_hash(&items[..k]);
            let right = merkle_hash(&items[k..]);
            inner(left, right)
        },
    }
}

fn leaf(data: &[u8]) -> [u8; 32] {
    let mut buf = vec![LEAF_PREFIX; 1];
    buf.extend(data);
    lhash::sha256(&buf)
}

fn inner(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
    let mut buf = vec![INNER_PREFIX; 1];
    buf.extend(left);
    buf.extend(right);
    lhash::sha256(&buf)
}

fn empty_hash() -> [u8; 32] {
    return lhash::sha256(&[]);
}

fn get_split_point(length: i64) -> i64 {
    if length < 1 {
        panic!("Trying to split a tree with size < 1");
    }
    let u_length = length as u64;
    let bitlen = 64 - u_length.leading_zeros();
    let mut k = 1 << (bitlen - 1);
    if k == length as u64 {
        k >>= 1;
    }
    k as i64
}

#[cfg(test)]
mod test {
    use crate::merkle::merkle_hash;

    #[test]
    /// Given a set of test vectors that are the same as on Tenderdash tests,
    /// When calculating the Merkle root hash,
    /// Then the result should be the same as the expected value generated in
    /// Tenderdash.
    ///
    /// ## See also
    ///
    /// Tenderdash test [TestHashFromByteSlices](https://github.com/dashpay/tenderdash/blob/feat/statesync-integration/crypto/merkle/tree_test.go#L21)
    fn test_merkle_root() {
        let items = vec![vec![1, 2], vec![3, 4], vec![5, 6], vec![7, 8], vec![9, 10]];
        let root = merkle_hash(&items);
        let expected = "f326493eceab4f2d9ffbc78c59432a0a005d6ea98392045c74df5d14a113be18";
        assert_eq!(hex::encode(root), expected)
    }
}
