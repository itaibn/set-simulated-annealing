
/**
Attempt to estimate the number of ways to partition a set deck into sets using
simulated annealing. More mathematically, this is the number of ways to
partition F_3^4 into affine lines.
**/

use std::collections::HashSet;
use rand::prelude::*;
use rand::distributions::Standard;
use once_cell::sync::Lazy;

const NUM_POINTS: usize = 81;
const NUM_LINES: usize = 81 * 80 / 6;
const PARTITION_SIZE: usize = 27;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Point(usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Line(usize);

const EMPTY: Line = Line(NUM_LINES);

static LINE: Lazy<[[Point; 3]; NUM_LINES]> = Lazy::new(|| {
    let mut line = [[Point(NUM_POINTS); 3]; NUM_LINES];
    let mut used_dir: HashSet<[usize; 4]> = HashSet::new();
    let mut index = 0;
    for dir_uint in 1..81usize {
        let dir = [dir_uint % 3,
                   (dir_uint / 3) % 3,
                   (dir_uint / 9) % 3,
                   dir_uint / 27];
        if used_dir.contains(&dir) {
            continue;
        }
        used_dir.insert(dir);
        used_dir.insert([(dir[0] * 2) % 3,
                         (dir[1] * 2) % 3,
                         (dir[2] * 2) % 3,
                         (dir[3] * 2) % 3]);
        let mut used_start: HashSet<[usize; 4]> = HashSet::new();
        for start_uint in 0..81 {
            let mut point = [start_uint % 3,
                             (start_uint / 3) % 3,
                             (start_uint / 9) % 3,
                             start_uint / 27];
            if used_start.contains(&point) {
                continue;
            }
            for i in 0..3 {
                line[index][i] = Point(point[0] + 3 * point[1] + 9 * point[2]
                    + 27 * point[3]);
                used_start.insert(point);
                point = [(point[0] + dir[0]) % 3,
                         (point[1] + dir[1]) % 3,
                         (point[2] + dir[2]) % 3,
                         (point[3] + dir[3]) % 3];
            }
            index += 1;
        }
        assert!(index % 27 == 0);
    }
    assert!(index == NUM_LINES);
    line
});

#[test]
fn test_line() {
    for line in &*LINE {
        for point in line {
            assert!(point.0 < NUM_POINTS);
        }
        assert!(line[0] != line[1] && line[0] != line[2] && line[1] != line[2]);
    }
    for point0 in 0..NUM_POINTS {
        for point1 in 0..NUM_POINTS {
            if point0 == point1 {
                continue;
            }
            let mut count = 0;
            for line in &*LINE {
                let mut has0 = false;
                let mut has1 = false;
                for p in line {
                    if p.0 == point0 {
                        has0 = true;
                    }
                    if p.0 == point1 {
                        has1 = true;
                    }
                }
                if has0 && has1 {
                    count += 1;
                }
            }
            assert!(count == 1);
        }
    }
}

/*
impl std::ops::Index<usize> for Line {
    type Output = Point;

    fn index(&self, index: usize) -> Point {
        Point((*LINE)[self.0])
    }
}
*/
impl Line {
    fn at(&self, index: usize) -> Point {
        (*LINE)[self.0][index]
    }
}

impl Distribution<Line> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Line {
        Line(rng.gen_range(0..NUM_LINES))
    }
}

struct Partition {
    line: [Line; PARTITION_SIZE],
    point_count: [i32; NUM_POINTS],
    // Necessary?
    cost: i32,
}

impl Default for Partition {
    fn default() -> Partition {
        Partition {
            line: [EMPTY; PARTITION_SIZE],
            point_count: [0; NUM_POINTS],
            cost: 0,
        }
    }
}

impl Distribution<Partition> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Partition {
        let mut partition = Partition::default();
        for i in 0..PARTITION_SIZE {
            partition.set(i, rng.gen());
        }
        partition
    }
}

impl Partition {
    fn count(&self, point: Point) -> &i32 {
        &self.point_count[point.0]
    }

    fn count_mut(&mut self, point: Point) -> &mut i32 {
        &mut self.point_count[point.0]
    }

    /// Set an empty index to a particular line, return the increase in cost
    fn set(&mut self, idx: usize, line: Line) -> i32 {
        assert!(self.line[idx] == EMPTY);
        self.line[idx] = line;

        let mut delta_cost = 0;
        for i in 0..3 {
            let count = self.count_mut(line.at(i));
            delta_cost += *count;
            *count += 1;
        }
        self.cost += delta_cost;
        delta_cost
    }

    fn cost_at(&self, line: Line) -> i32 {
        let mut delta_cost = 0;
        for i in 0..3 {
            let count = self.count(line.at(i));
            delta_cost += *count;
        }
        delta_cost
    }

    fn unset(&mut self, idx: usize) -> i32 {
        let line = self.line[idx];
        assert!(line.0 < NUM_LINES);
        self.line[idx] = EMPTY;

        let mut delta_cost = 0;
        for i in 0..3 {
            let count = self.count_mut(line.at(i));
            *count -= 1;
            delta_cost -= *count;
        }
        self.cost += delta_cost;
        delta_cost
    }

    fn boltzmann_line<R: Rng + ?Sized>(&self, rng: &mut R, beta: f64) -> Line {
        let mut z: f64 = 0.0;
        for i in 0..NUM_LINES {
            let line = Line(i);
            z += f64::exp(-beta * self.cost_at(line) as f64);
        }
        let mut sample = rng.gen_range(0.0..z);
        for i in 0..NUM_LINES {
            let line = Line(i);
            sample -= f64::exp(-beta * self.cost_at(line) as f64);
            if sample <= 0.0 {
                return line;
            }
        }
        unreachable!()
    }

    fn step<R: Rng + ?Sized>(&mut self, rng: &mut R, beta: f64) -> i32 {
        let idx = rng.gen_range(0..PARTITION_SIZE);
        let unset = self.unset(idx);
        unset + self.set(idx, self.boltzmann_line(rng, beta))
    }
}

fn main() {
    const ITER: u32 = 8_000;
    const MAX_BETA: f64 = 8.0;
    const DEL_BETA: f64 = MAX_BETA / ITER as f64;

    let mut rng = thread_rng();
    let mut beta: f64 = 0.0;
    let mut log_z: f64 = PARTITION_SIZE as f64 * f64::ln(NUM_LINES as f64);
    // Account for reordering ambiguity
    for n in 1 ..= PARTITION_SIZE {
        log_z -= f64::ln(n as f64);
    }
    let mut partition: Partition = rng.gen();
    for i in 0..ITER {
        /*let del_cost = */
            partition.step(&mut rng, beta);
        beta += DEL_BETA;
        log_z -= DEL_BETA * partition.cost as f64;
        if i % 100 == 0 {
            println!("Iter {}: beta {} cost {} log_z {}", i, beta,
                partition.cost, log_z);
        }
    }
    println!("z {:e}", f64::exp(log_z));
}
