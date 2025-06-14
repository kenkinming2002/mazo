#![feature(iterator_try_collect)]

pub mod binary_heap;

use rand::prelude::*;

use ratatui::{prelude::*, widgets::{Block, Paragraph}};
use layout::Position;
use style::Color;

use std::collections::{HashMap, HashSet};

use crossterm::event::*;

use crate::binary_heap::{BinaryHashHeap, BinaryHashHeapItem, PushAction};

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
    dimensions: Vec<usize>,

    start: Vec<usize>,
    end: Vec<usize>,

    position: Vec<usize>,
    axes: [usize; 2],

    walls: Vec<bool>,
}

impl Maze {
    pub fn new(dimensions: Vec<usize>) -> Maze {
        let start = vec![0; dimensions.len()];
        let end = vec![0; dimensions.len()];
        let position = vec![0; dimensions.len()];

        let axes = [0, 1];

        let wall_count = dimensions.iter().product::<usize>() * dimensions.len();
        let walls = vec![true; wall_count];

        Maze {
            dimensions,
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

        for (limit, value) in std::iter::zip(self.dimensions.iter(), wall.position.iter()) {
            index += stride * *value;
            stride *= *limit;
        }

        index += stride * wall.axis;
        index
    }

    /// Travel one square in the given axis in either positive or negative direction depending on
    /// given sign, wrapping around if necessary.
    pub fn traverse_inplace(&self, position: &mut [usize], axis: usize, sign: bool) {
        if sign {
            if position[axis] != self.dimensions[axis] - 1 {
                position[axis] += 1;
            } else {
                position[axis] = 0;
            }
        } else {
            if position[axis] != 0 {
                position[axis] -= 1;
            } else {
                position[axis] = self.dimensions[axis] - 1;
            }
        }
    }

    /// Travel one square in the given axis in either positive or negative direction depending on
    /// given sign, wrapping around if necessary.
    pub fn traverse(&self, position: &[usize], axis: usize, sign: bool) -> Vec<usize> {
        let mut result = position.to_vec();
        self.traverse_inplace(&mut result, axis, sign);
        result
    }

    /// Get the list of walls neighbouring a cell at position, together with the other cell.
    pub fn neighbours(&self, position: &[usize]) -> Vec<(Wall, Vec<usize>)> {
        let mut result = Vec::new();
        for axis in 0..self.dimensions.len() {
            for sign in [false, true] {
                let neighbour_position = self.traverse(position, axis, sign);
                let wall_position = if sign { position.to_vec() } else { neighbour_position.clone() };
                result.push((Wall { position: wall_position, axis, }, neighbour_position));
            }
        }
        result
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
        for (limit, value) in std::iter::zip(self.dimensions.iter(), self.start.iter_mut()) {
            *value = rng.random_range(0..*limit);
        }

        for (limit, value) in std::iter::zip(self.dimensions.iter(), self.end.iter_mut()) {
            *value = rng.random_range(0..*limit);
        }

        // Yep. This waste a lot of memory, but apparently who cares?
        let mut visited = HashSet::<Vec<usize>>::from_iter([self.start.clone()]);
        let mut walls = Wall::from_cell(&self.dimensions, &self.start);

        self.reset_walls();
        while !walls.is_empty() {
            let wall = walls.swap_remove(rng.random_range(0..walls.len()));

            let mut okay = false;
            for cell in wall.get_neighbour_cells(&self.dimensions) {
                if !visited.contains(&cell) {
                    for wall in Wall::from_cell(&self.dimensions, &cell) {
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

    /// Compute the taxicab distance between two positions but take into account the fact that we
    /// are on a torus.
    fn distance(&self, position1: &[usize], position2: &[usize]) -> usize {
        let mut result : usize = 0;
        for (i, dimension) in self.dimensions.iter().enumerate() {
            result += (position1[i] as isize - position2[i] as isize).div_euclid(*dimension as isize) as usize;
        }
        result
    }

    pub fn solve(&mut self) -> Vec<Vec<usize>> {
        #[derive(Debug)]
        struct Node {
            g_score: usize,
            f_score: usize,
            position: Vec<usize>,
        }

        impl BinaryHashHeapItem for Node {
            type Key = Vec<usize>;
            type Value = usize;

            fn key(&self) -> &Self::Key {
                &self.position
            }

            fn value(&self) -> &Self::Value {
                &self.f_score
            }
        }

        let mut open = BinaryHashHeap::default();
        open.push(PushAction::Keep, Node {
            position: self.start.clone(),
            g_score: 0,
            f_score: self.distance(&self.start, &self.end)
        });

        let mut visited = HashSet::new();
        let mut links = HashMap::new();

        while let Some(node) = open.pop() {
            if node.position == self.end {
                let mut paths = Vec::new();

                let mut current = self.end.clone();
                while current != self.start {
                    let next = links.remove(&current).unwrap();
                    paths.push(current);
                    current = next;
                }

                paths.push(current);
                paths.reverse();
                return paths;
            }

            for (wall, neighbour_position) in self.neighbours(&node.position) {
                if visited.contains(&neighbour_position) {
                    continue;
                }

                if self.get_wall(&wall) {
                    continue;
                }

                let g_score = node.g_score + 1;
                let f_score = g_score + self.distance(&neighbour_position, &self.end);
                if !open.push(PushAction::DecreaseKey, Node {
                    position: neighbour_position.clone(),
                    g_score, f_score,
                }) {
                    continue;
                }

                links.insert(neighbour_position, node.position.clone());
            }

            visited.insert(node.position);
        }

        panic!("No path found")
    }

    pub fn start(&mut self) {
        self.position.copy_from_slice(&self.start);
    }

    pub fn walk(&mut self, view_axis: usize, sign: bool) {
        if sign {
            if self.get_wall(&Wall { position: self.position.clone(), axis: self.axes[view_axis] }) {
                return;
            }

            if self.position[self.axes[view_axis]] != self.dimensions[self.axes[view_axis]] - 1 {
                self.position[self.axes[view_axis]] += 1;
            } else {
                self.position[self.axes[view_axis]] = 0;
            }
        } else {
            let old_value = self.position[self.axes[view_axis]];

            if self.position[self.axes[view_axis]] != 0 {
                self.position[self.axes[view_axis]] -= 1;
            } else {
                self.position[self.axes[view_axis]] = self.dimensions[self.axes[view_axis]] - 1;
            }

            if self.get_wall(&Wall { position: self.position.clone(), axis: self.axes[view_axis] }) {
                self.position[self.axes[view_axis]] = old_value;
            }
        }
    }

    pub fn set_view_axis(&mut self, view_axis: usize, axis : usize) {
        if view_axis < 2 && axis < self.dimensions.len() {
            self.axes[view_axis] = axis;
        }
    }
}

enum Application {
    Menu {
        dimension: String,
    },
    Main {
        maze: Maze,
        view_axis : Option<usize>,
        solution: Option<Vec<Vec<usize>>>,
    },
}

fn render_maze(area: Rect, buf: &mut Buffer, maze: &Maze, solution: Option<&Vec<Vec<usize>>>) {
    let height = area.height;
    let width = area.width / 2;

    let solution = solution
        .iter()
        .copied()
        .flatten()
        .cloned()
        .enumerate()
        .map(|(i, p)| (p, (i % 100) as u8) )
        .collect::<HashMap<_, _>>();

    for y in 0..height {
        for x in 0..width {
            let wy = y as isize - (height / 2) as isize;
            let wx = x as isize - (width / 2) as isize;

            enum RenderCell {
                Wall,
                Empty,
                Start,
                End,
                Current,
                Solution(u8),
            }

            match match (wy.rem_euclid(2), wx.rem_euclid(2)) {
                (1, 1) => RenderCell::Wall,
                (ry, rx)  => {
                    let mut position = maze.position.clone();
                    position[maze.axes[0]] = (position[maze.axes[0]] as isize + wy.div_euclid(2)).rem_euclid(maze.dimensions[maze.axes[0]] as isize) as usize;
                    position[maze.axes[1]] = (position[maze.axes[1]] as isize + wx.div_euclid(2)).rem_euclid(maze.dimensions[maze.axes[1]] as isize) as usize;
                    match (ry, rx) {
                        (0, 0) => {
                            if position == maze.start {
                                RenderCell::Start
                            } else if position == maze.end {
                                RenderCell::End
                            } else if position == maze.position {
                                RenderCell::Current
                            } else if let Some(i) = solution.get(&position) {
                                RenderCell::Solution(*i)
                            } else {
                                RenderCell::Empty
                            }
                        },
                        (1, 0) => if maze.get_wall(&Wall { position, axis: maze.axes[0] }) { RenderCell::Wall } else { RenderCell::Empty },
                        (0, 1) => if maze.get_wall(&Wall { position, axis: maze.axes[1] }) { RenderCell::Wall } else { RenderCell::Empty },
                        _ => unreachable!(),
                    }
                },
            } {
                RenderCell::Wall => {
                    buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char('█');
                    buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char('█');
                },
                RenderCell::Empty => {
                    buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char(' ');
                    buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char(' ');
                },
                RenderCell::Start => {
                    buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char('█').set_fg(Color::Green);
                    buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char('█').set_fg(Color::Green);
                },
                RenderCell::End => {
                    buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char('█').set_fg(Color::Red);
                    buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char('█').set_fg(Color::Red);
                },
                RenderCell::Current => {
                    buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char('█').set_fg(Color::Yellow);
                    buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char('█').set_fg(Color::Yellow);
                },
                RenderCell::Solution(i) => {
                    buf[Position { x: area.x + x * 2 + 0, y : area.y + y }].set_char(char::from_digit((i / 10) as u32, 10).unwrap()).set_fg(Color::Cyan);
                    buf[Position { x: area.x + x * 2 + 1, y : area.y + y }].set_char(char::from_digit((i % 10) as u32, 10).unwrap()).set_fg(Color::Cyan);
                },
            }
        }
    }
}

fn parse_dimension(s: &str) -> Option<Vec<usize>> {
    s
        .split(',')
        .map(|s| s.trim())
        .map(|s| s.parse())
        .try_collect()
        .ok()
}

impl Application {
    pub fn new() -> Application {
        Self::Menu { dimension: String::new() }
    }

    pub fn run(&mut self) {
        let mut terminal = ratatui::init();
        loop {
            terminal.draw(|frame| self.render(frame)).unwrap();
            if !self.update() {
                break
            }
        }
        ratatui::restore();
    }

    pub fn render(&self, frame: &mut Frame) {
        match self {
            Application::Menu { dimension } => {
                let text = if dimension.is_empty() {
                    Text::from(" Enter dimension of maze to be generated here: (e.g. 50, 40, 30) ").style(Style::new().dark_gray())
                } else {
                    if parse_dimension(dimension).is_some() {
                        Text::from(format!(" Dimension: {dimension} ")).style(Style::new().green())
                    } else {
                        Text::from(format!(" Dimension: {dimension} ")).style(Style::new().red())
                    }
                };

                let desired_width = (text.width() + 2) as u16;
                let desired_height = 3;

                let mut input_area = frame.area();

                if input_area.width > desired_width {
                    input_area.x += (input_area.width - desired_width) / 2;
                    input_area.width = desired_width;
                }

                if input_area.height > desired_height {
                    input_area.y += (input_area.height - desired_height) / 2;
                    input_area.height = desired_height;
                }

                let input_widget = Paragraph::new(text).block(Block::bordered());
                frame.render_widget(input_widget, input_area);
            },
            Application::Main { maze, view_axis, solution } => {
                let mut info = Text::default();

                {
                    let mut line = Line::default();
                    line.push_span("Current Axes (Vertical, Horizontal): ");

                    let mut span = Span::raw(format!("{}", maze.axes[0]));
                    if *view_axis == Some(0) { span = span.style(Style::new().red()); }
                    line.push_span(span);

                    line.push_span(" ");

                    let mut span = Span::raw(format!("{}", maze.axes[1]));
                    if *view_axis == Some(1) { span = span.style(Style::new().red()); }
                    line.push_span(span);

                    info.push_line(line);
                }

                {
                    let mut line = Line::default();
                    line.push_span("Dimensions: ");
                    for (i, dimension) in maze.dimensions.iter().enumerate() {
                        if i != 0 { line.push_span(", "); }
                        line.push_span(dimension.to_string());
                    }
                    info.push_line(line);
                }

                {
                    let mut line = Line::default();
                    line.push_span("Position: ");
                    for (i, dimension) in maze.position.iter().enumerate() {
                        if i != 0 { line.push_span(", "); }
                        line.push_span(dimension.to_string());
                    }
                    info.push_line(line);
                }

                {
                    let mut line = Line::default();
                    line.push_span("Start: ");
                    for (i, dimension) in maze.start.iter().enumerate() {
                        if i != 0 { line.push_span(", "); }
                        line.push_span(dimension.to_string());
                    }
                    info.push_line(line);
                }

                {
                    let mut line = Line::default();
                    line.push_span("End: ");
                    for (i, dimension) in maze.end.iter().enumerate() {
                        if i != 0 { line.push_span(", "); }
                        line.push_span(dimension.to_string());
                    }
                    info.push_line(line);
                }

                let mut help = Text::default();

                match view_axis {
                    Some(_) => {
                        let mut line = Line::default();
                        line.push_span(format!("0-{}: Select replacement axis", maze.dimensions.len()-1));
                        help.push_line(line);

                        let mut line = Line::default();
                        line.push_span("Esc: Cancel selection of replacment axis");
                        help.push_line(line);

                    },
                    None => {
                        let mut line = Line::default();
                        line.push_span("0-1: Select which axes to modify");
                        help.push_line(line);

                        let mut line = Line::default();
                        line.push_span("Esc: exit");
                        help.push_line(line);
                    },
                }

                match solution {
                    Some(_) => {
                        let mut line = Line::default();
                        line.push_span("s: Unsolve maze");
                        help.push_line(line);
                    },
                    None => {
                        let mut line = Line::default();
                        line.push_span("s: Solve maze");
                        help.push_line(line);
                    },
                }

                {
                    let mut line = Line::default();
                    line.push_span("Arrow Keys: Move");
                    help.push_line(line);
                }

                let [info_area, help_area, maze_area] = Layout::vertical([
                    Constraint::Length((info.lines.len()+2).try_into().unwrap()),
                    Constraint::Length((help.lines.len()+2).try_into().unwrap()),
                    Constraint::Min(0),
                ]).areas(frame.area());

                let info_block = Block::bordered().title("Info");
                let help_block = Block::bordered().title("Help");

                frame.render_widget(&info_block, info_area);
                frame.render_widget(&info, info_block.inner(info_area));

                frame.render_widget(&help_block, help_area);
                frame.render_widget(&help, help_block.inner(help_area));

                render_maze(maze_area, frame.buffer_mut(), maze, solution.as_ref());
            },
        }
    }

    pub fn update(&mut self) -> bool {
        let event = read().unwrap();
        match event {
            Event::Key(key_event) => match key_event {
                KeyEvent { code : KeyCode::Char('c'), modifiers : KeyModifiers::CONTROL, .. } => return false,
                KeyEvent { code : KeyCode::Char('q'), .. } => return false,
                _ => {},
            },
            _ => {},
        };

        match self {
            Application::Menu { dimension } => {
                match event {
                    Event::Key(key_event) => match key_event {
                        KeyEvent { code : KeyCode::Char(c), .. } => { dimension.push(c); },
                        KeyEvent { code : KeyCode::Esc, .. } => { dimension.clear(); },
                        KeyEvent { code : KeyCode::Backspace, .. } => { dimension.pop(); },
                        KeyEvent { code : KeyCode::Enter, .. } => {
                            if let Some(dimension) = parse_dimension(dimension) {
                                let mut maze = Maze::new(dimension);
                                maze.generate(&mut rand::rng());
                                maze.start();
                                *self = Application::Main { maze, view_axis : None, solution: None }
                            }
                        },
                        _ => {},
                    },
                    _ => {},
                }
            },
            Application::Main { maze, view_axis, solution } => {
                match event {
                    Event::Key(key_event) => match key_event {
                        KeyEvent { code : KeyCode::Up, .. } => maze.walk(0, false),
                        KeyEvent { code : KeyCode::Down, .. } => maze.walk(0, true),
                        KeyEvent { code : KeyCode::Left, .. } => maze.walk(1, false),
                        KeyEvent { code : KeyCode::Right, .. } => maze.walk(1, true),

                        KeyEvent { code : KeyCode::Esc, .. } => {
                            match view_axis {
                                Some(_) => *view_axis = None,
                                None => *self = Application::new(),
                            }
                        },

                        KeyEvent { code : KeyCode::Char(c), .. } if c >= '0' && c <= '9' => {
                            let d = c as usize - '0' as usize;
                            match view_axis.take() {
                                Some(view_axis) => maze.set_view_axis(view_axis, d),
                                None => *view_axis = Some(d),
                            }
                        },

                        KeyEvent { code : KeyCode::Char('s'), .. } => {
                            if solution.take().is_none() {
                                *solution = Some(maze.solve());
                            }
                        },

                        _ => {},
                    },
                    _ => {},
                }
            },
        }

        true
    }
}

fn main() {
    Application::new().run()
}
