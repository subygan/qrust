use std::mem::swap;

use crate::{
    codewords::Codewords,
    constants::{FORMAT_INFO, VERSION_INFO},
    qrcode::{Mask, Version, ECL},
};
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

// todo, should be possible to set branchless
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Clone, Copy, PartialEq)]
pub enum Module {
    DataOFF,
    DataON,

    FinderOFF,
    FinderON,

    AlignmentOFF,
    AlignmentON,

    TimingOFF,
    TimingON,

    FormatOFF,
    FormatON,

    VersionOFF,
    VersionON,

    Separator,
    Unset,
}

pub struct Matrix {
    pub value: Vec<Module>,
    pub width: usize,
    pub version: Version,
    pub ecl: ECL,
    pub mask: Mask,
}

impl Matrix {
    pub fn new(codewords: Codewords, mask: Option<Mask>) -> Self {
        let width = codewords.version.0 * 4 + 17;
        let mut matrix = Matrix {
            width,
            value: vec![Module::Unset; width * width],
            version: codewords.version,
            ecl: codewords.ecl,
            mask: if let Some(mask) = mask {
                mask
            } else {
                Mask::M0
            },
        };
        place_all(&mut matrix, &codewords);

        if let None = mask {
            let mut min_score = score(&matrix);
            let mut min_mask = matrix.mask;
            for m in [
                Mask::M1,
                Mask::M2,
                Mask::M3,
                Mask::M4,
                Mask::M5,
                Mask::M6,
                Mask::M7,
            ] {
                // undo prev mask
                apply_mask(&mut matrix);

                matrix.mask = m;
                apply_mask(&mut matrix);
                place_format(&mut matrix);
                let score = score(&matrix);
                if score < min_score {
                    min_score = score;
                    min_mask = matrix.mask;
                }
            }
            if min_mask != matrix.mask {
                // undo prev mask
                apply_mask(&mut matrix);

                matrix.mask = min_mask;
                apply_mask(&mut matrix);

                place_format(&mut matrix);
            }
        }

        matrix
    }
    fn set(&mut self, x: usize, y: usize, module: Module) {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = x * self.width + y;
        self.value[i] = module;
    }
    pub fn get(&self, x: usize, y: usize) -> Module {
        let i = x * self.width + y;
        self.value[i]
    }
}

fn place_all(matrix: &mut Matrix, codewords: &Codewords) {
    place_finder(matrix);
    place_format(matrix);
    place_timing(matrix);
    place_version(matrix);
    place_alignment(matrix);
    place_data(matrix, codewords);
    apply_mask(matrix);
}

fn place_finder(matrix: &mut Matrix) {
    fn place_pattern(matrix: &mut Matrix, col: usize, mut row: usize) {
        for i in 0..7 {
            matrix.set(col + i, row, Module::FinderON);
        }
        row += 1;

        matrix.set(col + 0, row, Module::FinderON);
        for i in 1..6 {
            matrix.set(col + i, row, Module::FinderOFF);
        }
        matrix.set(col + 6, row, Module::FinderON);
        row += 1;

        for _ in 0..3 {
            matrix.set(col + 0, row, Module::FinderON);
            matrix.set(col + 1, row, Module::FinderOFF);
            matrix.set(col + 2, row, Module::FinderON);
            matrix.set(col + 3, row, Module::FinderON);
            matrix.set(col + 4, row, Module::FinderON);
            matrix.set(col + 5, row, Module::FinderOFF);
            matrix.set(col + 6, row, Module::FinderON);
            row += 1;
        }

        matrix.set(col + 0, row, Module::FinderON);
        for i in 1..6 {
            matrix.set(col + i, row, Module::FinderOFF);
        }
        matrix.set(col + 6, row, Module::FinderON);
        row += 1;

        for i in 0..7 {
            matrix.set(col + i, row, Module::FinderON);
        }
    }

    place_pattern(matrix, 0, 0);
    for i in 0..8 {
        matrix.set(i, 7, Module::Separator);
    }
    for i in 0..7 {
        matrix.set(7, i, Module::Separator);
    }

    let width = matrix.width;
    place_pattern(matrix, 0, width - 7);
    for i in 0..8 {
        matrix.set(i, matrix.width - 8, Module::Separator);
    }
    for i in 0..7 {
        matrix.set(7, matrix.width - 1 - i, Module::Separator);
    }

    place_pattern(matrix, width - 7, 0);
    for i in 0..8 {
        matrix.set(matrix.width - 1 - i, 7, Module::Separator);
    }
    for i in 0..7 {
        matrix.set(matrix.width - 8, i, Module::Separator);
    }
}

fn place_format(matrix: &mut Matrix) {
    let format_info = FORMAT_INFO[matrix.ecl as usize][matrix.mask as usize];
    for i in 0..15 {
        let module = if ((format_info >> i) & 1) == 1 {
            Module::FormatON
        } else {
            Module::FormatOFF
        };

        let y = match i {
            i if i < 6 => i,
            6 => 7,
            _ => 8,
        };
        let x = match i {
            i if i < 8 => 8,
            8 => 7,
            _ => 14 - i,
        };
        matrix.set(x, y, module);

        let y = match i {
            i if i < 8 => 8,
            _ => matrix.width - (15 - i),
        };
        let x = match i {
            i if i < 8 => matrix.width - (i + 1),
            _ => 8,
        };
        matrix.set(x, y, module);
    }

    // always set
    matrix.set(8, matrix.width - 8, Module::FormatON);
}

fn place_version(matrix: &mut Matrix) {
    if matrix.version.0 < 7 {
        return;
    }
    let info = VERSION_INFO[matrix.version.0];

    for i in 0..18 {
        let module = if (info >> i) & 1 == 1 {
            Module::VersionON
        } else {
            Module::VersionOFF
        };

        let x = i / 3;
        let y = i % 3;

        matrix.set(x, y + matrix.width - 11, module);
        matrix.set(y + matrix.width - 11, x, module);
    }
}

fn place_timing(matrix: &mut Matrix) {
    let len = matrix.width - 16;
    for i in 0..len {
        let module = if i & 1 == 0 {
            Module::TimingON
        } else {
            Module::TimingOFF
        };
        matrix.set(8 + i, 6, module);
        matrix.set(6, 8 + i, module);
    }
}

const ALIGN_COORDS: [usize; 34] = [
    16, 18, 20, 22, 24, 26, 28, // 7-13
    20, 22, 24, 24, 26, 28, 28, // 14-20
    22, 24, 24, 26, 26, 28, 28, // 21-27
    24, 24, 26, 26, 26, 28, 28, // 28-34
    24, 26, 26, 26, 28, 28, // 35-40
];

fn place_alignment(matrix: &mut Matrix) {
    let version = matrix.version.0;
    if version == 1 {
        return;
    }

    let first = 6;
    let last = matrix.width - 7;
    let len = version / 7 + 2;
    let mut coords = Vec::with_capacity(len);

    coords.push(first);
    if version >= 7 {
        for i in (1..len - 1).rev() {
            coords.push(last - i * ALIGN_COORDS[version - 7]);
        }
    }
    coords.push(last);

    for i in 0..len {
        for j in 0..len {
            if (i == 0 && (j == 0 || j == len - 1)) || (i == len - 1 && j == 0) {
                continue;
            }

            let col = coords[i] - 2;
            let row = coords[j] - 2;

            for i in 0..5 {
                matrix.set(col, row + i, Module::AlignmentON)
            }

            for i in 1..4 {
                matrix.set(col + i, row, Module::AlignmentON);
                matrix.set(col + i, row + 1, Module::AlignmentOFF);
                matrix.set(col + i, row + 2, Module::AlignmentOFF);
                matrix.set(col + i, row + 3, Module::AlignmentOFF);
                matrix.set(col + i, row + 4, Module::AlignmentON);
            }

            matrix.set(col + 2, row + 2, Module::AlignmentON);

            for i in 0..5 {
                matrix.set(col + 4, row + i, Module::AlignmentON)
            }
        }
    }
}

// This depends on all placements occuring beforehand
fn place_data(matrix: &mut Matrix, qrcode: &Codewords) {
    fn place_module(
        matrix: &mut Matrix,
        qrcode: &Codewords,
        col: usize,
        row: usize,
        i: &mut usize,
    ) {
        if matrix.get(col, row) == Module::Unset {
            // FOR FUTURE KYLE
            // i means ith data bit
            // qrcode.value[*i / 8] gets current codeword aka byte
            // 7 - (*i % 8) gets the current bit position in codeword (greatest to least order)
            // & 1 to check if set and XOR with mask
            // in c could just use value directly b/c DataOn = 1, DataOFF = 0, but oh well
            let module = if ((qrcode.value[*i / 8] >> (7 - (*i % 8))) & 1) == 1 {
                Module::DataON
            } else {
                Module::DataOFF
            };
            *i += 1;

            matrix.set(col, row, module);
        }
    }

    let mut i = 0;

    let mut col = matrix.width - 1;
    let mut row = matrix.width - 1;

    loop {
        loop {
            place_module(matrix, qrcode, col, row, &mut i);
            place_module(matrix, qrcode, col - 1, row, &mut i);
            if row == 0 {
                break;
            }
            row -= 1;
        }

        col -= 2;
        // col 6 is vertical timing belt
        if col == 6 {
            col -= 1;
        }

        loop {
            place_module(matrix, qrcode, col, row, &mut i);
            place_module(matrix, qrcode, col - 1, row, &mut i);
            if row == matrix.width - 1 {
                break;
            }
            row += 1;
        }

        if col == 1 {
            break;
        }
        col -= 2;
    }
}

fn apply_mask(matrix: &mut Matrix) {
    let mask_bit = match matrix.mask {
        Mask::M0 => |row: usize, col: usize| (row + col) % 2 == 0,
        Mask::M1 => |row: usize, _: usize| (row) % 2 == 0,
        Mask::M2 => |_: usize, col: usize| (col) % 3 == 0,
        Mask::M3 => |row: usize, col: usize| (row + col) % 3 == 0,
        Mask::M4 => |row: usize, col: usize| ((row / 2) + (col / 3)) % 2 == 0,
        Mask::M5 => |row: usize, col: usize| ((row * col) % 2 + (row * col) % 3) == 0,
        Mask::M6 => |row: usize, col: usize| ((row * col) % 2 + (row * col) % 3) % 2 == 0,
        Mask::M7 => |row: usize, col: usize| ((row + col) % 2 + (row * col) % 3) % 2 == 0,
    };

    for i in 0..matrix.width {
        for j in 0..matrix.width {
            let module = matrix.get(i, j) as u8;
            if module | 1 != Module::DataON as u8 {
                continue;
            }

            matrix.set(
                i,
                j,
                if module ^ mask_bit(i, j) as u8 == 1 {
                    Module::DataON
                } else {
                    Module::DataOFF
                },
            );
        }
    }
}

// todo UNTESTED CODE: HERE BE DRAGONS
// if score is wrong for all masks, then this still works
fn score(matrix: &Matrix) -> usize {
    // todo are cache misses even a concern? vec is small

    fn dark_proportion(matrix: &Matrix) -> usize {
        let dark = matrix
            .value
            .iter()
            .filter(|m| **m == Module::DataON)
            .count();

        let mut percent = (dark * 20) / (20 * matrix.width * matrix.width);
        let mut middle = 50;
        if percent < middle {
            swap(&mut percent, &mut middle);
        }
        let k = (percent - middle) / 5;
        10 * k
    }

    fn blocks(matrix: &Matrix) -> usize {
        let mut score = 0;
        for i in 0..matrix.width - 1 {
            for j in 0..matrix.width - 1 {
                let curr = matrix.get(i, j) as u8 & 1;
                let tr = matrix.get(i + 1, j) as u8 & 1;
                let bl = matrix.get(i, j + 1) as u8 & 1;
                let br = matrix.get(i + 1, j + 1) as u8 & 1;
                if curr == tr && curr == bl && curr == br {
                    score += 3;
                }
            }
        }
        score
    }

    // detects streaks >= 5 and finder patterns
    fn line_patterns(matrix: &Matrix, col: bool) -> usize {
        let mut score = 0;
        let (i_mult, j_mult) = match col {
            true => (matrix.width, 1),
            false => (1, matrix.width),
        };

        let pattern_1 = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1];
        let pattern_2 = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0];

        for i in 0..matrix.width {
            let mut streak = 1;
            let mut streak_v = matrix.value[i * i_mult + 0] as u8 & 1;

            let mut finder_i = if streak_v == pattern_1[0] { 1 } else { 0 };
            let mut finder_j = if streak_v == pattern_2[0] { 1 } else { 0 };

            for j in 1..matrix.width {
                let curr = matrix.value[i * i_mult + j * j_mult] as u8 & 1;

                if curr == streak_v {
                    streak += 1;
                    if streak == 5 {
                        score += 3;
                    } else if streak > 5 {
                        score += 1;
                    }
                } else {
                    streak = 0;
                    streak_v = curr;
                }

                if curr == pattern_1[finder_i] {
                    finder_i += 1;
                    if finder_i == pattern_1.len() {
                        score += 40;
                        finder_i = 0;
                    }
                } else {
                    finder_i = 0;
                }
                if curr == pattern_2[finder_j] {
                    finder_j += 1;
                    if finder_j == pattern_2.len() {
                        score += 40;
                        finder_j = 0;
                    }
                } else {
                    finder_j = 0;
                }
            }
        }

        score
    }

    dark_proportion(matrix)
        + blocks(matrix)
        + line_patterns(matrix, true)
        + line_patterns(matrix, false)
}
