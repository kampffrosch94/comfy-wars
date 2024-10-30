// vendored
#![allow(dead_code)]
use crate::comfy_compat::*;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Index, IndexMut, Mul};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid<T> {
    pub data: Vec<T>,
    pub width: i32,
    pub height: i32,
}

impl<T> Default for Grid<T> {
    fn default() -> Self {
        Self { data: Default::default(), width: Default::default(), height: Default::default() }
    }
}


impl<T: Clone> Grid<T> {
    pub fn new(width: i32, height: i32, value: T) -> Self {
        Self {
            data: vec![value; (width * height) as usize],
            width,
            height,
        }
    }

    pub fn filled_with<F: FnMut(i32, i32) -> T>(width: i32, height: i32, mut f: F) -> Self {
        let mut data = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                data.push(f(x, y));
            }
        }

        Self {
            data,
            width,
            height,
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn is_valid(&self, coord: IVec2) -> bool {
        coord.x >= 0 && coord.x < self.width() && coord.y >= 0 && coord.y < self.height()
    }

    pub fn get(&self, x: i32, y: i32) -> &T {
        &self[(x, y)]
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> &mut T {
        &mut self[(x, y)]
    }

    pub fn get_clamped(&self, x: i32, y: i32) -> &T {
        let x = x.clamp(0, self.width - 1);
        let y = y.clamp(0, self.height - 1);

        self.get(x, y)
    }

    pub fn get_clamped_v(&self, v: IVec2) -> &T {
        let x = v.x;
        let y = v.y;

        let x = x.clamp(0, self.width - 1);
        let y = y.clamp(0, self.height - 1);

        self.get(x, y)
    }

    pub fn v_clamped(&self, v: Vec2) -> &T {
        let x = v.x as i32;
        let y = v.y as i32;

        let x = x.clamp(0, self.width - 1);
        let y = y.clamp(0, self.height - 1);

        self.get(x, y)
    }

    pub fn get_clamped_mut(&mut self, x: i32, y: i32) -> &mut T {
        let x = x.clamp(0, self.width - 1);
        let y = y.clamp(0, self.height - 1);

        self.get_mut(x, y)
    }

    pub fn iter_rect(&self, min: IVec2, max: IVec2) -> impl Iterator<Item = (i32, i32, &T)> {
        let mut coords = vec![];

        for x in min.x..max.x {
            for y in min.y..max.y {
                coords.push((x, y));
            }
        }

        coords.into_iter().map(|(x, y)| (x, y, &self[(x, y)]))
    }

    pub fn clone_rect(&self, min: IVec2, max: IVec2) -> Grid<T> {
        let dims = max - min;
        let mut result = Grid::new(dims.x, dims.y, self[(0i32, 0i32)].clone());

        for x in 0..dims.x {
            for y in 0..dims.y {
                result[(x, y)] = self[(x + min.x, y + min.y)].clone();
            }
        }

        result
    }

    pub fn paste_grid(&mut self, offset: IVec2, other: &Grid<T>) {
        for x in 0..other.width() {
            for y in 0..other.height() {
                self[(offset.x + x, offset.y + y)] = other[(x, y)].clone();
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (i32, i32, &T)> {
        self.data.iter().enumerate().map(|(i, v)| {
            let i = i as i32;
            let x = i % self.width;
            let y = i / self.width;

            (x, y, v)
        })
    }

    pub fn iter_values(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    pub fn into_iter_values(self) -> impl Iterator<Item = T> {
        self.data.into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (i32, i32, &mut T)> {
        self.data.iter_mut().enumerate().map(|(i, v)| {
            let i = i as i32;
            let x = i % self.width;
            let y = i / self.height;

            (x, y, v)
        })
    }

    pub fn iter_values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }

    pub fn iter_coords(&self) -> impl Iterator<Item = (IVec2, &T)> {
        self.iter().map(|(x, y, v)| (IVec2::new(x, y), v))
    }

    pub fn iter_coords_mut(&mut self) -> impl Iterator<Item = (IVec2, &mut T)> {
        self.iter_mut().map(|(x, y, v)| (IVec2::new(x, y), v))
    }

    pub fn coords(&self) -> Vec<IVec2> {
        self.iter()
            .map(|(x, y, _)| IVec2::new(x, y))
            .collect::<Vec<_>>()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn size(&self) -> IVec2 {
        IVec2::new(self.width(), self.height())
    }

    pub fn pack(fill: T, grids: Vec<Grid<T>>) -> Grid<T> {
        assert!(grids.len() > 0);

        let mut size = IVec2::ZERO;

        for grid in grids.iter() {
            size.x += grid.width();
            size.y = size.y.max(grid.height());
        }

        let mut result = Grid::new(size.x, size.y, fill.clone());

        let mut offset = IVec2::ZERO;

        for grid in grids.iter() {
            for (x, y, val) in grid.iter() {
                result[offset + IVec2::new(x, y)] = val.clone();
            }

            offset += IVec2::new(grid.width(), 0);
        }

        result
    }

    /// Returns an iterator over the rows of the grid.
    ///
    /// ```
    /// use grids::Grid;
    ///
    /// let mut grid = Grid::new(3, 2, 0);
    ///
    /// grid[(0, 0)] = 1;
    /// grid[(1, 0)] = 2;
    /// grid[(2, 0)] = 3;
    /// grid[(0, 1)] = 5;
    /// grid[(1, 1)] = 6;
    /// grid[(2, 1)] = 7;
    ///
    /// let mut row_iter = grid.row_iter();
    /// assert_eq!(row_iter.next().unwrap().cloned().collect::<Vec<_>>(), vec![1, 2, 3]);
    /// assert_eq!(row_iter.next().unwrap().cloned().collect::<Vec<_>>(), vec![5, 6, 7]);
    /// assert!(row_iter.next().is_none()
    /// )
    ///
    /// ```
    pub fn row_iter(&self) -> impl Iterator<Item = core::slice::Iter<'_, T>> {
        (0..self.height).map(move |y| {
            let start = (y * self.width) as usize;
            let end = start + self.width as usize;
            self.data[start..end].iter()
        })
    }

    /// asserts that both grids have the same dimensions
    /// panics if they don't
    pub fn ensure_dimensions_match<R>(&self, other: &Grid<R>) {
        assert_eq!(
            self.width, other.width,
            "Grids need the same width {} vs {}",
            self.width, other.width
        );
        assert_eq!(
            self.height, other.height,
            "Grids need the same height {} vs {}",
            self.height, other.height
        );
    }

    /// multiplies each value in the grid with each value at the same
    /// coordinate in the other grid
    /// and returns a new grid, leaving the parameters untouched
    /// if types differ the type of the lhs is converted to the type of the rhs
    ///
    /// panics if dimensions don't match
    pub fn mul<R, O>(&self, other: &Grid<R>) -> Grid<O>
    where
        T: Into<R>,
        R: Clone + Mul<R, Output = O>,
    {
        self.ensure_dimensions_match(other);
        let mut data: Vec<O> = Vec::with_capacity(self.data.len());
        for (lhs, rhs) in self.data.iter().zip(other.data.iter()) {
            let lhs: R = lhs.clone().into();
            data.push(lhs.clone().mul(rhs.clone()));
        }
        Grid {
            data,
            width: self.width,
            height: self.height,
        }
    }

    /// multiplies each value in the grid with each value at the same
    /// coordinate in the other grid
    /// modifies the grid in place
    ///
    /// panics if dimensions don't match
    pub fn mul_inplace<R>(&mut self, other: &Grid<R>) -> &mut Self
    where
        T: Mul<R, Output = T>,
        R: Clone,
    {
        self.ensure_dimensions_match(other);
        for (lhs, rhs) in self.data.iter_mut().zip(other.data.iter()) {
            *lhs = lhs.clone().mul(rhs.clone());
        }
        self
    }

    /// adds each value in the grid with each value at the same
    /// coordinate in the other grid
    /// and returns a new grid, leaving the parameters untouched
    ///
    /// panics if dimensions don't match
    pub fn add<R, O>(&self, other: &Grid<R>) -> Grid<O>
    where
        T: Into<R>,
        R: Clone + Add<R, Output = O>,
    {
        self.ensure_dimensions_match(other);
        let mut data: Vec<O> = Vec::with_capacity(self.data.len());
        for (lhs, rhs) in self.data.iter().zip(other.data.iter()) {
            let lhs: R = lhs.clone().into();
            data.push(lhs.add(rhs.clone()));
        }
        Grid {
            data,
            width: self.width,
            height: self.height,
        }
    }

    /// adds each value in the grid with each value at the same
    /// coordinate in the other grid
    /// modifies the grid in place
    ///
    /// panics if dimensions don't match
    pub fn add_inplace<R>(&mut self, other: &Grid<R>) -> &mut Self
    where
        T: Add<R, Output = T>,
        R: Clone,
    {
        self.ensure_dimensions_match(other);
        for (lhs, rhs) in self.data.iter_mut().zip(other.data.iter()) {
            *lhs = lhs.clone().add(rhs.clone());
        }
        self
    }

    /// multiplies each value in the grid with the scalar
    /// and returns a new grid, leaving the old one untouched
    /// if types differ the type of the lhs is converted to the type of the rhs
    pub fn mul_scalar<R, O>(&self, scalar: R) -> Grid<O>
    where
        T: Into<R>,
        R: Clone + Mul<R, Output = O>,
    {
        let mut data: Vec<O> = Vec::with_capacity(self.data.len());
        for lhs in self.data.iter() {
            let lhs: R = lhs.clone().into();
            data.push(lhs.mul(scalar.clone()));
        }
        Grid {
            data,
            width: self.width,
            height: self.height,
        }
    }

    /// clamps all values in the grid, so that
    /// `min <= value <= max`
    pub fn clamp_values(&mut self, min: T, max: T)
    where
        T: PartialOrd<T>,
    {
        for value in self.data.iter_mut() {
            if (*value).lt(&min) {
                *value = min.clone();
            } else if (*value).gt(&max) {
                *value = max.clone();
            }
        }
    }
}

impl<T: Clone, Pos: Into<(i32, i32)>> Index<Pos> for Grid<T> {
    type Output = T;

    fn index(&self, pos: Pos) -> &Self::Output {
        let (x, y) = pos.into();
        &self.data[(x + y * self.width) as usize]
    }
}

impl<T: Clone, Pos: Into<(i32, i32)>> IndexMut<Pos> for Grid<T> {
    fn index_mut(&mut self, pos: Pos) -> &mut Self::Output {
        let (x, y) = pos.into();
        &mut self.data[(x + y * self.width) as usize]
    }
}

// impl<T: Clone> Index<(u32, u32)> for Grid<T> {
//     type Output = T;
//
//     fn index(&self, (x, y): (u32, u32)) -> &Self::Output {
//         &self.data[(x as i32 + y as i32 * self.width) as usize]
//     }
// }
//
// impl<T: Clone> IndexMut<(u32, u32)> for Grid<T> {
//     fn index_mut(&mut self, (x, y): (u32, u32)) -> &mut Self::Output {
//         &mut self.data[(x as i32 + y as i32 * self.width) as usize]
//     }
// }

#[test]
fn test_stuff() {
    let mut grid = Grid::new(3, 2, 0);
    grid[(0, 1)] = 5;

    assert_eq!(grid[IVec2::new(1, 0)], 0);

    assert_eq!(
        grid.into_iter_values().collect::<Vec<_>>(),
        vec![0, 0, 0, 5, 0, 0]
    );
}

#[test]
fn readme_test() {
    let mut grid = Grid::new(3, 2, 0); // A 3x2 grid filled with zeros.
    grid[(0, 1)] = 5;

    // Accessing using IVec2.
    assert_eq!(grid[IVec2::new(1, 0)], 0);

    // Converting grid to a Vec.
    assert_eq!(
        grid.into_iter_values().collect::<Vec<_>>(),
        vec![0, 0, 0, 5, 0, 0]
    );
}

#[test]
fn test_row_iter_empty_grid() {
    let grid: Grid<i32> = Grid::new(0, 0, 0);
    let mut row_iter = grid.row_iter();

    assert!(row_iter.next().is_none());
}

#[test]
fn test_row_iter_single_row() {
    let grid = Grid::new(3, 1, 42);
    let mut row_iter = grid.row_iter();

    if let Some(row) = row_iter.next() {
        let row: Vec<_> = row.cloned().collect();
        assert_eq!(row, vec![42, 42, 42]);
    } else {
        panic!("Expected one row, but got none");
    }

    assert!(row_iter.next().is_none());
}

#[test]
fn test_row_iter_multiple_rows() {
    let grid = Grid::filled_with(3, 3, |x, y| x + y);
    let mut row_iter = grid.row_iter();

    #[rustfmt::skip]
    let expected_rows = vec![
        vec![0, 1, 2],
        vec![1, 2, 3],
        vec![2, 3, 4]
    ];

    for (i, expected_row) in expected_rows.iter().enumerate() {
        if let Some(row) = row_iter.next() {
            let row: Vec<_> = row.cloned().collect();
            assert_eq!(&row, expected_row, "Row {} did not match", i);
        } else {
            panic!("Expected more rows, but got none");
        }
    }

    assert!(row_iter.next().is_none(), "Expected no more rows");
}

#[test]
fn test_grid_mul() {
    let mut mask_grid = Grid::filled_with(3, 3, |x, y| if y > 0 { 0 } else { x + 9 });
    let mut value_grid = Grid::filled_with(3, 3, |x, y| x + y);

    mask_grid.clamp_values(0, 1);
    let result = value_grid.mul(&mask_grid);

    #[rustfmt::skip]
    let expected_data = vec![
        0, 1, 2,
        0, 0, 0,
        0, 0, 0
    ];

    assert_eq!(expected_data, result.data);

    // test inplace multiplication too
    assert_ne!(expected_data, value_grid.data);
    value_grid.mul_inplace(&mask_grid);
    assert_eq!(expected_data, value_grid.data);

    // also scalar multiplication while changing the grid type from int to float
    let doubled_float = value_grid.mul_scalar(2.0);
    assert_eq!(4., doubled_float[(2, 0)]);

    // multiplying with a float grid
    let float_grid = Grid::new(3, 3, 1.5);
    let value_grid_float = value_grid.mul(&float_grid);
    assert_eq!(3., value_grid_float[(2, 0)]);
}

#[test]
fn test_grid_add() {
    let mut a = Grid::new(3, 3, 1);
    let b = Grid::new(3, 3, 2);
    let c = a.add(&b);
    assert_eq!(3, c[(2, 2)]);

    // inplace
    a.add_inplace(&b);
    assert_eq!(3, a[(2, 2)]);

    // add float
    let b = Grid::new(3, 3, 0.5);
    let c = a.add(&b);
    assert_eq!(3.5, c[(2, 2)]);
}
