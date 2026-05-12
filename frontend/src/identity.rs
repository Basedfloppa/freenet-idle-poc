//! Browser-side seed candidate for first-ever contact with the
//! delegate. The delegate stores the very first seed it sees and
//! ignores everything we send it afterward — so this value is only
//! relevant when the node has no identity yet for this delegate.

pub fn random_seed_candidate() -> [u8; 32] {
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).expect("browser getrandom");
    buf
}
