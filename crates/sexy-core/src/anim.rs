//! Cel animation (`AnimInfo`). Spec: `docs/re/assets/images.md#cel-animation-animinfo`.

use crate::resources::{Anim, AnimType, ImageRes};

#[derive(Debug, Clone, PartialEq)]
pub struct AnimInfo {
    pub kind: AnimType,
    pub frame_delay: i32,
    pub num_cels: i32,
    pub per_frame_delay: Vec<i32>,
    pub frame_map: Vec<i32>,
    pub total_time: i32,
}

impl AnimInfo {
    /// `AnimInfo::Compute`. `num_cels` is `max(rows, cols)` in the original.
    pub fn compute(anim: &Anim, num_cels: i32) -> Self {
        let mut a = AnimInfo {
            kind: anim.kind,
            frame_delay: anim.frame_delay.max(1),
            num_cels: num_cels.max(1),
            per_frame_delay: anim.per_frame_delay.clone(),
            frame_map: anim.frame_map.clone(),
            total_time: 0,
        };
        if a.kind == AnimType::PingPong && a.num_cels > 1 && a.frame_map.is_empty() {
            let n = a.num_cels;
            a.frame_map = (0..n).chain((1..n - 1).rev()).collect();
        }
        if !a.frame_map.is_empty() {
            a.num_cels = a.frame_map.len() as i32;
        }
        let n = a.num_cels as usize;
        if anim.begin_delay > 0 {
            set_grow(&mut a.per_frame_delay, 0, anim.begin_delay);
        }
        // TODO(re): `enddelay` (not used by shipped data) would set index n-1.
        if a.per_frame_delay.is_empty() {
            a.total_time = a.num_cels * a.frame_delay;
        } else {
            a.per_frame_delay.resize(n, 0);
            for d in &mut a.per_frame_delay {
                if *d < 1 {
                    *d = a.frame_delay;
                }
            }
            a.total_time = a.per_frame_delay.iter().sum();
        }
        a
    }

    pub fn for_image(res: &ImageRes) -> Option<Self> {
        (res.anim.kind != AnimType::None)
            .then(|| Self::compute(&res.anim, res.rows.max(res.cols) as i32))
    }

    /// `AnimInfo::GetCel`. May return `-1` from a frame map ("draw nothing").
    pub fn cel(&self, time: i32) -> i32 {
        if self.kind == AnimType::Once && time >= self.total_time {
            return self.frame_map.last().copied().unwrap_or(self.num_cels - 1);
        }
        let i = if self.per_frame_delay.is_empty() {
            ((time % self.total_time) / self.frame_delay) % self.num_cels
        } else {
            // TODO(re): the original walks with `time` as given; confirm
            // whether it is reduced modulo total_time first.
            let mut t = time % self.total_time;
            let mut idx = self.num_cels - 1;
            for (k, d) in self.per_frame_delay.iter().enumerate() {
                t -= d;
                if t < 0 {
                    idx = k as i32;
                    break;
                }
            }
            idx
        };
        if self.frame_map.is_empty() {
            i
        } else {
            self.frame_map[i as usize]
        }
    }
}

fn set_grow(v: &mut Vec<i32>, i: usize, val: i32) {
    if v.len() <= i {
        v.resize(i + 1, 0);
    }
    v[i] = val;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anim(kind: AnimType, frame_delay: i32) -> Anim {
        Anim {
            kind,
            frame_delay,
            ..Default::default()
        }
    }

    #[test]
    fn loop_cycles() {
        let a = AnimInfo::compute(&anim(AnimType::Loop, 2), 10);
        assert_eq!(a.total_time, 20);
        let cels: Vec<_> = (0..24).map(|t| a.cel(t)).collect();
        assert_eq!(&cels[..6], &[0, 0, 1, 1, 2, 2]);
        assert_eq!(cels[19], 9);
        assert_eq!(cels[20], 0);
    }

    #[test]
    fn pingpong_builds_map() {
        let a = AnimInfo::compute(&anim(AnimType::PingPong, 1), 4);
        assert_eq!(a.frame_map, [0, 1, 2, 3, 2, 1]);
        let cels: Vec<_> = (0..8).map(|t| a.cel(t)).collect();
        assert_eq!(cels, [0, 1, 2, 3, 2, 1, 0, 1]);
    }

    #[test]
    fn once_holds_last_mapped_frame() {
        // MERYLBLINK: once, framemap 0,1,2,1,0,-1, perframedelay 3,2,3,1,1,1
        let a = AnimInfo::compute(
            &Anim {
                kind: AnimType::Once,
                frame_map: vec![0, 1, 2, 1, 0, -1],
                per_frame_delay: vec![3, 2, 3, 1, 1, 1],
                ..Default::default()
            },
            3,
        );
        assert_eq!(a.total_time, 11);
        let cels: Vec<_> = (0..12).map(|t| a.cel(t)).collect();
        assert_eq!(cels, [0, 0, 0, 1, 1, 2, 2, 2, 1, 0, -1, -1]);
        assert_eq!(a.cel(100), -1);
    }

    #[test]
    fn begin_delay_extends_first_frame() {
        let a = AnimInfo::compute(
            &Anim {
                kind: AnimType::Loop,
                frame_delay: 2,
                begin_delay: 3,
                ..Default::default()
            },
            3,
        );
        assert_eq!(a.per_frame_delay, [3, 2, 2]);
        assert_eq!(a.total_time, 7);
    }
}
