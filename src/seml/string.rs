//! Small string helpers ported from `../seml/src/string.ts`. Kept regex-free
//! (manual char checks) to avoid pulling in a `regex` dependency.

/// Accept only non-negative whole numbers (`^\d+$`), no leading `+`.
pub fn parse_natural(s: &str) -> Option<i32> {
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse::<i32>().ok()
    } else {
        None
    }
}

/// Accept only non-negative whole or decimal numbers (`^\d+(\.\d+)?$`), no `+`.
pub fn parse_decimal(s: &str) -> Option<f64> {
    let valid = match s.split_once('.') {
        None => !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()),
        Some((int, frac)) => {
            !int.is_empty()
                && int.bytes().all(|b| b.is_ascii_digit())
                && !frac.is_empty()
                && frac.bytes().all(|b| b.is_ascii_digit())
        }
    };
    if valid {
        s.parse::<f64>().ok()
    } else {
        None
    }
}

pub fn chop_prefix<'a>(s: &'a str, prefix: &str) -> &'a str {
    s.strip_prefix(prefix).unwrap_or(s)
}

pub fn chop_suffix<'a>(s: &'a str, suffix: &str) -> &'a str {
    s.strip_suffix(suffix).unwrap_or(s)
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=a.len()).collect();
    let mut curr = vec![0usize; a.len() + 1];
    for i in 1..=b.len() {
        curr[0] = i;
        for j in 1..=a.len() {
            curr[j] = if b[i - 1] == a[j - 1] {
                prev[j - 1]
            } else {
                1 + prev[j - 1].min(curr[j - 1]).min(prev[j])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[a.len()]
}

/// Closest match to `query` among `candidates`, or `None` if all are completely
/// different (distance >= max possible). Mirrors `findClosestString`.
pub fn find_closest_string<'a>(query: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let q: Vec<char> = query.chars().collect();
    let mut best: Option<&str> = None;
    let mut best_dist = usize::MAX;
    for cand in candidates {
        let c: Vec<char> = cand.chars().collect();
        let dist = levenshtein(&q, &c);
        let max_possible = q.len().max(c.len());
        if dist < max_possible && dist < best_dist {
            best_dist = dist;
            best = Some(cand);
        }
    }
    best
}
