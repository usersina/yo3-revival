//! Release admission shared by discovery, account requests and key authentication.
fn parts(version: &str) -> Option<Vec<u64>> {
    let version=version.strip_prefix('v').unwrap_or(version).split('-').next()?;
    let numbers:Vec<u64>=version.split('.').map(str::parse).collect::<Result<_,_>>().ok()?;
    (!numbers.is_empty() && numbers.len()<=3).then_some(numbers)
}

pub fn version_at_least(have: &str, need: &str) -> bool {
    let (Some(have),Some(need))=(parts(have),parts(need)) else {return false};
    (0..3).map(|i|(have.get(i).copied().unwrap_or(0),need.get(i).copied().unwrap_or(0)))
        .find(|(h,n)|h!=n).is_none_or(|(h,n)|h>n)
}

/// Hosts may raise the floor, never lower it below their own release. Unversioned local image builds have no floor.
pub fn minimum_launcher<'a>(release: &'a str, configured: Option<&'a str>) -> Option<&'a str> {
    let release=release.strip_prefix('v').unwrap_or(release).split('-').next().unwrap_or(release);
    if parts(release).is_none() {return configured}
    Some(configured.filter(|need|version_at_least(need,release)).unwrap_or(release))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn releases_set_a_floor_and_settings_cannot_lower_it() {
        assert_eq!(minimum_launcher("v0.1.1",None),Some("0.1.1"));
        assert_eq!(minimum_launcher("v0.1.1",Some("0.1.0")),Some("0.1.1"));
        assert_eq!(minimum_launcher("v0.1.1",Some("0.2.0")),Some("0.2.0"));
        assert_eq!(minimum_launcher("0.1.1-dev",None),Some("0.1.1"));
        assert!(!version_at_least("0.1.0","0.1.1"));
        assert!(version_at_least("0.1.1","0.1.1"));
        for bad in ["","junk","0.x.1","0.1.1.1"] {assert!(!version_at_least(bad,"0.1.1"));}
    }
}
