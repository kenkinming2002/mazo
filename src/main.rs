use rand::prelude::*;

use std::io::prelude::*;
use std::collections::HashSet;

use crossterm::event::*;
use crossterm::terminal::*;

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

struct Maze {
    shape: Vec<usize>,

    start: Vec<usize>,
    end: Vec<usize>,

    position: Vec<usize>,
    axes: [usize; 2],

    walls: Vec<bool>,
}

impl Maze {
    pub fn new(shape: Vec<usize>) -> Maze {
        let start = vec![0; shape.len()];
        let end = vec![0; shape.len()];
        let position = vec![0; shape.len()];

        let axes = [0, 1];

        let wall_count = shape.iter().product::<usize>() * shape.len();
        let walls = vec![true; wall_count];

        Maze {
            shape,
            start,
            end,
            position,
            axes,
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

    pub fn start(&mut self) {
        self.position.copy_from_slice(&self.start);
    }

    pub fn walk(&mut self, view_axis: usize, sign: bool) {
        if sign {
            if self.get_wall(&Wall { position: self.position.clone(), axis: self.axes[view_axis] }) {
                return;
            }

            if self.position[self.axes[view_axis]] != self.shape[self.axes[view_axis]] - 1 {
                self.position[self.axes[view_axis]] += 1;
            } else {
                self.position[self.axes[view_axis]] = 0;
            }
        } else {
            let old_value = self.position[self.axes[view_axis]];

            if self.position[self.axes[view_axis]] != 0 {
                self.position[self.axes[view_axis]] -= 1;
            } else {
                self.position[self.axes[view_axis]] = self.shape[self.axes[view_axis]] - 1;
            }

            if self.get_wall(&Wall { position: self.position.clone(), axis: self.axes[view_axis] }) {
                self.position[self.axes[view_axis]] = old_value;
            }
        }
    }

    pub fn render(&self) {
        let (mut width, height) = size().unwrap();
        width /= 2;

        let mut stdout = std::io::stdout();

        crossterm::queue!(&mut stdout, BeginSynchronizedUpdate).unwrap();
        crossterm::queue!(&mut stdout, Clear(ClearType::All)).unwrap();
        crossterm::queue!(&mut stdout, crossterm::cursor::MoveTo(0, 0)).unwrap();

        for y in 0..height {
            for x in 0..width {
                let y = y as isize - (height / 2) as isize;
                let x = x as isize - (width / 2) as isize;

                enum RenderCell {
                    Wall,
                    Empty,
                    Start,
                    End,
                    Current,
                }

                match match (y.rem_euclid(2), x.rem_euclid(2)) {
                    (1, 1) => RenderCell::Wall,
                    (ry, rx)  => {
                        let mut position = self.position.clone();
                        position[self.axes[0]] = (position[self.axes[0]] as isize + y.div_euclid(2)).rem_euclid(self.shape[self.axes[0]] as isize) as usize;
                        position[self.axes[1]] = (position[self.axes[1]] as isize + x.div_euclid(2)).rem_euclid(self.shape[self.axes[1]] as isize) as usize;
                        match (ry, rx) {
                            (0, 0) => {
                                if position == self.start {
                                    RenderCell::Start
                                } else if position == self.end {
                                    RenderCell::End
                                } else if position == self.position {
                                    RenderCell::Current
                                } else {
                                    RenderCell::Empty
                                }
                            },
                            (1, 0) => if self.get_wall(&Wall { position, axis: 0 }) { RenderCell::Wall } else { RenderCell::Empty },
                            (0, 1) => if self.get_wall(&Wall { position, axis: 1 }) { RenderCell::Wall } else { RenderCell::Empty },
                            _ => unreachable!(),
                        }
                    },
                } {
                    RenderCell::Wall => write!(&mut stdout, "██").unwrap(),
                    RenderCell::Empty => write!(&mut stdout, "  ").unwrap(),
                    RenderCell::Start => {
                        crossterm::queue!(&mut stdout, crossterm::style::SetForegroundColor(crossterm::style::Color::Green)).unwrap();
                        write!(&mut stdout, "██").unwrap();
                        crossterm::queue!(&mut stdout, crossterm::style::SetForegroundColor(crossterm::style::Color::White)).unwrap();
                    },
                    RenderCell::End => {
                        crossterm::queue!(&mut stdout, crossterm::style::SetForegroundColor(crossterm::style::Color::Red)).unwrap();
                        write!(&mut stdout, "██").unwrap();
                        crossterm::queue!(&mut stdout, crossterm::style::SetForegroundColor(crossterm::style::Color::White)).unwrap();
                    },
                    RenderCell::Current => {
                        crossterm::queue!(&mut stdout, crossterm::style::SetForegroundColor(crossterm::style::Color::Yellow)).unwrap();
                        write!(&mut stdout, "██").unwrap();
                        crossterm::queue!(&mut stdout, crossterm::style::SetForegroundColor(crossterm::style::Color::White)).unwrap();
                    },
                }
            }
            write!(&mut stdout, "\r\n").unwrap();
        }

        crossterm::queue!(&mut stdout, EndSynchronizedUpdate).unwrap();

        stdout.flush().unwrap();
    }
}

fn main() {
    let mut maze = Maze::new(vec![30, 30]);
    maze.generate(&mut rand::rng());
    maze.start();

    enable_raw_mode().unwrap();
    crossterm::execute!(std::io::stdout(), EnterAlternateScreen).unwrap();

    loop {
        maze.render();
        match read().unwrap() {
            Event::Key(key_event) => match key_event {
                KeyEvent { code : KeyCode::Up, .. } => maze.walk(0, false),
                KeyEvent { code : KeyCode::Down, .. } => maze.walk(0, true),
                KeyEvent { code : KeyCode::Left, .. } => maze.walk(1, false),
                KeyEvent { code : KeyCode::Right, .. } => maze.walk(1, true),
                KeyEvent { code : KeyCode::Char('c'), modifiers : KeyModifiers::CONTROL, .. } => break,
                KeyEvent { code : KeyCode::Char('q'), .. } => break,
                _ => {},
            },
            _ => {},
        }
    }

    crossterm::execute!(std::io::stdout(), LeaveAlternateScreen).unwrap();
}
