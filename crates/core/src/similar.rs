//! 似ているパスワードの判定（同じものは「使い回し」なので除く）
//! - 1〜2文字だけ違う（例 Sample2023! と Sample2024!）
//! - 数字や記号を取ると同じ（例 kumo#1 と kumo-77、置き換え文字 @→a 0→o なども戻して比べる）
//! 比べるのはメモリの中だけ。結果に残るのは「どの項目と似ているか」だけ。

use zeroize::Zeroizing;

/// 比べるための形（小文字）
pub fn norm(pw: &str) -> Zeroizing<String> {
    Zeroizing::new(pw.to_lowercase())
}

/// 数字・記号を取った骨組み。leet=true なら、よくある置き換え文字（@→a 0→o など）を戻してから取る
pub fn skeleton(n: &str, leet: bool) -> Zeroizing<String> {
    let mut s = String::with_capacity(n.len());
    for c in n.chars() {
        let c = if !leet {
            c
        } else {
            match c {
                '@' | '4' => 'a',
                '0' => 'o',
                '3' => 'e',
                '1' | '!' => 'i',
                '$' | '5' => 's',
                '7' => 't',
                _ => c,
            }
        };
        if c.is_alphabetic() {
            s.push(c);
        }
    }
    Zeroizing::new(s)
}

/// 文字の差（何文字直せば同じになるか）。max を超えたら途中でやめる
pub fn distance(a: &str, b: &str, max: usize) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return max + 1;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        let mut row_min = cur[0];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            row_min = row_min.min(cur[j]);
        }
        if row_min > max {
            return max + 1;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// 2つのパスワード（norm 済み）が似ているか。まったく同じなら false
pub fn is_similar(a: &str, b: &str) -> bool {
    if a == b {
        return false;
    }
    let (la, lb) = (a.chars().count(), b.chars().count());
    if la >= 6 && lb >= 6 && la <= 40 && lb <= 40 && distance(a, b, 2) <= 2 {
        return true;
    }
    [false, true].into_iter().any(|leet| {
        let (sa, sb) = (skeleton(a, leet), skeleton(b, leet));
        sa.chars().count() >= 5 && *sa == *sb
    })
}

/// 似ている同士をまとめる。戻り値は「まとまり」ごとの番号の一覧（2件以上のものだけ）
pub fn groups(pws: &[Option<Zeroizing<String>>]) -> Vec<Vec<usize>> {
    let n = pws.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut y = x;
        while p[y] != r {
            let nx = p[y];
            p[y] = r;
            y = nx;
        }
        r
    }
    for i in 0..n {
        let Some(a) = &pws[i] else { continue };
        for j in (i + 1)..n {
            let Some(b) = &pws[j] else { continue };
            if is_similar(a, b) {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[rj] = ri;
                }
            }
        }
    }
    let mut map: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for i in 0..n {
        if pws[i].is_some() {
            let r = find(&mut parent, i);
            map.entry(r).or_default().push(i);
        }
    }
    map.into_values().filter(|v| v.len() > 1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn similar() {
        assert!(is_similar(&norm("Sample2023!"), &norm("Sample2024!")));
        assert!(is_similar(&norm("kumoyama#1"), &norm("kumoyama-77")));
        assert!(is_similar(&norm("P@ssw0rd-umi"), &norm("password_umi99")));
        assert!(
            !is_similar(&norm("Sample2023!"), &norm("Sample2023!")),
            "同じは使い回し"
        );
        assert!(!is_similar(
            &norm("q8$Lz!vR2#pW9@tM6&kN"),
            &norm("Zq8$Lr!vT2#pW9@kM6&n4")
        ));
        assert!(
            !is_similar(&norm("abc1"), &norm("abc2")),
            "短すぎるものは比べない"
        );
        assert_eq!(distance("kitten", "sitting", 5), 3);
        let g = groups(&[
            Some(norm("Sample2023!")),
            Some(norm("zzzzzzzzzzzzzzzz")),
            Some(norm("Sample2024!")),
            None,
            Some(norm("sample2025!")),
        ]);
        assert_eq!(g, vec![vec![0, 2, 4]]);
    }
}
