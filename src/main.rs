use rand::prelude::*;
use std::collections::HashSet;

struct Maze {
    shape: Vec<usize>,
    start: Vec<usize>,
    end: Vec<usize>,
    walls: Vec<bool>,
}

impl Maze {
    pub fn new(shape: Vec<usize>) -> Maze {
        let start = vec![0; shape.len()];
        let end = vec![0; shape.len()];

        let wall_count = shape.iter().product::<usize>() * shape.len();
        let walls = vec![true; wall_count];

        Maze {
            shape,
            start,
            end,
            walls,
        }
    }

    pub fn compute_wall_index(&self, wall: &Wall) -> usize {
        let mut index = 0;
        let mut stride = 1;

        for (limit, value) in std::iter::zip(self.shape.iter(), wall.position.iter()) {
            index += stride * *value;
            stride *= *limit;
        }

        index += stride * wall.axis;
        index
    }

    pub fn reset_walls(&mut self) {
        self.walls.fill(true);
    }

    pub fn get_wall(&self, wall: &Wall) -> bool {
        let index = self.compute_wall_index(wall);
        self.walls[index]
    }

    pub fn set_wall(&mut self, wall: &Wall, value: bool) {
        let index = self.compute_wall_index(wall);
        self.walls[index] = value;
    }

    pub fn generate<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        for (limit, value) in std::iter::zip(self.shape.iter(), self.start.iter_mut()) {
            *value = rng.random_range(0..*limit);
        }

        for (limit, value) in std::iter::zip(self.shape.iter(), self.end.iter_mut()) {
            *value = rng.random_range(0..*limit);
        }

        // Yep. This waste a lot of memory, but apparently who cares?
        let mut visited = HashSet::<Vec<usize>>::from_iter([self.start.clone()]);
        let mut walls = Wall::from_cell(&self.shape, &self.start);

        self.reset_walls();
        while !walls.is_empty() {
            let wall = walls.swap_remove(rng.random_range(0..walls.len()));

            let mut okay = false;
            for cell in wall.get_neighbour_cells(&self.shape) {
                if !visited.contains(&cell) {
                    for wall in Wall::from_cell(&self.shape, &cell) {
                        if self.get_wall(&wall) {
                            walls.push(wall);
                        }
                    }
                    visited.insert(cell);
                    okay = true;
                }
            }

            if okay {
                self.set_wall(&wall, false);
            }
        }
    }

    pub fn display2d(&self) {
        assert_eq!(self.shape.len(), 2);
        for y in 0..self.shape[0] {
            for x in 0..self.shape[1] {
                if y == self.start[0] && x == self.start[1] {
                    print!("s");
                } else if y == self.end[0] && x == self.end[1] {
                    print!("e");
                } else {
                    print!(" ");
                }

                if self.get_wall(&Wall { position : vec![y, x], axis: 1 } ) {
                    print!("█");
                } else {
                    print!(" ");
                }
            }
            println!("");

            for x in 0..self.shape[1] {
                if self.get_wall(&Wall { position : vec![y, x], axis: 0 } ) {
                    print!("█");
                } else {
                    print!(" ");
                }
                print!("█");
            }
            println!("");
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct Wall {
    position: Vec<usize>,
    axis: usize,
}

impl Wall {
    /// Get the list of walls neighbouring a cell at position.
    pub fn from_cell(shape: &[usize], position: &[usize]) -> Vec<Wall> {
        let mut walls = Vec::new();
        for axis in 0..shape.len() {
            for sign in [false, true] {
                let mut position = position.to_vec();
                if sign {
                    if position[axis] != 0 {
                        position[axis] -= 1;
                    } else {
                        position[axis] = shape[axis] - 1;
                    }
                }
                walls.push(Wall { position, axis });
            }
        }
        walls
    }

    /// Get the two cells neighbouring the wall.
    pub fn get_neighbour_cells(&self, shape: &[usize]) -> [Vec<usize>; 2] {
        let position1 = self.position.clone();

        let mut position2 = self.position.clone();
        if position2[self.axis] != shape[self.axis] - 1 {
            position2[self.axis] += 1;
        } else {
            position2[self.axis] = 0;
        }

        [position1, position2]
    }
}

fn main() {
    let mut maze = Maze::new(vec![20, 60]);
    maze.generate(&mut rand::rng());
    maze.display2d();
}
