use smallvec::{SmallVec, smallvec};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

pub trait Value<V> {
    fn value(&self) -> V;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn to_scroll_direction(self) -> ScrollDirection {
        match self {
            Axis::Horizontal => ScrollDirection::Horizontal,
            Axis::Vertical => ScrollDirection::Vertical,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScrollDirection {
    Horizontal,
    Vertical,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Clip {
    None,
    Rect,
    RoundedRect { border_radius: BorderRadius },
    Oval,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClipShape {
    Rect,
    RoundedRect { border_radius: BorderRadius },
    Oval,
}

impl Clip {
    pub fn to_shape(self) -> Option<ClipShape> {
        match self {
            Clip::None => None,
            Clip::Rect => Some(ClipShape::Rect),
            Clip::RoundedRect { border_radius } => Some(ClipShape::RoundedRect { border_radius }),
            Clip::Oval => Some(ClipShape::Oval),
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum LayoutDirection {
    #[default]
    LTR,
    RTL,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Size {
    pub width: SizeConstraint,
    pub height: SizeConstraint,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum SizeConstraint {
    Fill(f32),
    #[default]
    Wrap,
    Fixed(f64),
}

impl SizeConstraint {
    pub fn constrained(&self) -> bool {
        match self {
            SizeConstraint::Fill(_) => true,
            SizeConstraint::Wrap => false,
            SizeConstraint::Fixed(_) => true,
        }
    }
}

impl From<Vec2<f64>> for Size {
    fn from(value: Vec2<f64>) -> Self {
        Size::fixed(value.x, value.y)
    }
}

impl From<f32> for SizeConstraint {
    fn from(value: f32) -> Self {
        SizeConstraint::Fixed(value as f64)
    }
}

impl From<f64> for SizeConstraint {
    fn from(value: f64) -> Self {
        SizeConstraint::Fixed(value)
    }
}

impl Size {
    pub fn new(width: SizeConstraint, height: SizeConstraint) -> Self {
        Self { width, height }
    }

    pub fn fixed(width: f64, height: f64) -> Self {
        Self {
            width: SizeConstraint::Fixed(width),
            height: SizeConstraint::Fixed(height),
        }
    }

    pub fn fill() -> Self {
        Self {
            width: SizeConstraint::Fill(1.0),
            height: SizeConstraint::Fill(1.0),
        }
    }

    pub fn wrap() -> Self {
        Self {
            width: SizeConstraint::Wrap,
            height: SizeConstraint::Wrap,
        }
    }

    pub fn square(size: impl Into<SizeConstraint>) -> Self {
        let constraint = size.into();

        Self {
            width: constraint,
            height: constraint,
        }
    }
}

impl From<f32> for Size {
    fn from(value: f32) -> Self {
        Self {
            width: SizeConstraint::Fixed(value as f64),
            height: SizeConstraint::Fixed(value as f64),
        }
    }
}

impl From<f64> for Size {
    fn from(value: f64) -> Self {
        Self {
            width: SizeConstraint::Fixed(value),
            height: SizeConstraint::Fixed(value),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum AlignX {
    Left,
    Right,
    #[default]
    Start,
    End,
    Center,

    /// Custom fractional alignment.
    ///
    /// Range: -1.0 (left/start) to 1.0 (right/end), with 0.0 being center.
    Fraction(f32),
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum AlignY {
    #[default]
    Top,
    Bottom,
    Center,

    /// Custom fractional alignment.
    ///
    /// Range: -1.0 (top) to 1.0 (bottom), with 0.0 being center.
    Fraction(f32),
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    #[default]
    Auto,
    Left,
    Right,
    Center,
    End,
    Justified,
}

impl TextAlign {
    pub(crate) fn to_align_x(self) -> AlignX {
        match self {
            TextAlign::Auto => AlignX::Start,
            TextAlign::Left => AlignX::Left,
            TextAlign::Right => AlignX::Right,
            TextAlign::Center => AlignX::Center,
            TextAlign::End => AlignX::End,
            TextAlign::Justified => AlignX::Start,
        }
    }
}

impl AlignX {
    #[inline]
    pub fn position_f64(&self, layout_direction: LayoutDirection, boundary: f64, size: f64) -> f64 {
        match self {
            AlignX::Left => 0.,
            AlignX::Right => boundary - size,
            AlignX::Center => (boundary - size) / 2.,
            AlignX::Start => match layout_direction {
                LayoutDirection::LTR => 0.,
                LayoutDirection::RTL => boundary - size,
            },
            AlignX::End => match layout_direction {
                LayoutDirection::LTR => boundary - size,
                LayoutDirection::RTL => 0.,
            },
            AlignX::Fraction(fraction) => {
                // Convert -1.0..1.0 to 0.0..1.0
                let normalized = (*fraction as f64 + 1.0) / 2.0;

                normalized * (boundary - size)
            }
        }
    }

    #[inline]
    pub fn position_f32(&self, layout_direction: LayoutDirection, boundary: f32, size: f32) -> f32 {
        match self {
            AlignX::Left => 0.,
            AlignX::Right => boundary - size,
            AlignX::Center => (boundary - size) / 2.,
            AlignX::Start => match layout_direction {
                LayoutDirection::LTR => 0.,
                LayoutDirection::RTL => boundary - size,
            },
            AlignX::End => match layout_direction {
                LayoutDirection::LTR => boundary - size,
                LayoutDirection::RTL => 0.,
            },
            AlignX::Fraction(fraction) => {
                // Convert -1.0..1.0 to 0.0..1.0
                let normalized = (fraction + 1.0) / 2.0;

                normalized * (boundary - size)
            }
        }
    }
}

impl AlignY {
    #[inline]
    pub fn position_f64(&self, boundary: f64, size: f64) -> f64 {
        match self {
            AlignY::Top => 0.,
            AlignY::Bottom => boundary - size,
            AlignY::Center => (boundary - size) / 2.,
            AlignY::Fraction(fraction) => {
                // Convert -1.0..1.0 to 0.0..1.0
                let normalized = (*fraction as f64 + 1.0) / 2.0;

                normalized * (boundary - size)
            }
        }
    }

    #[inline]
    pub fn position_f32(&self, boundary: f32, size: f32) -> f32 {
        match self {
            AlignY::Top => 0.,
            AlignY::Bottom => boundary - size,
            AlignY::Center => (boundary - size) / 2.,
            AlignY::Fraction(fraction) => {
                // Convert -1.0..1.0 to 0.0..1.0
                let normalized = (fraction + 1.0) / 2.0;

                normalized * (boundary - size)
            }
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainAxisAlignment {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossAxisAlignment {
    #[default]
    Start,
    End,
    Center,
    Stretch,
    Baseline,
}

#[derive(Default, Debug, Clone, PartialEq, Copy)]
pub struct EdgeInsets {
    pub top: f64,
    pub left: f64,
    pub right: f64,
    pub bottom: f64,
}

impl EdgeInsets {
    /// An [`EdgeInsets`] with all sides set to zero.
    pub const ZERO: Self = Self {
        top: 0.0,
        left: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    pub fn new() -> Self {
        Self::ZERO
    }

    pub fn left(self, value: f64) -> Self {
        Self {
            top: self.top,
            left: value,
            right: self.right,
            bottom: self.bottom,
        }
    }

    pub fn top(self, value: f64) -> Self {
        Self {
            top: value,
            left: self.left,
            right: self.right,
            bottom: self.bottom,
        }
    }

    pub fn right(self, value: f64) -> Self {
        Self {
            top: self.top,
            left: self.left,
            right: value,
            bottom: self.bottom,
        }
    }

    pub fn bottom(self, value: f64) -> Self {
        Self {
            top: self.top,
            left: self.left,
            right: self.right,
            bottom: value,
        }
    }

    /// Creates a new [`EdgeInsets`] with all sides set to the same value.
    ///
    /// This is a convenience method for creating [`EdgeInsets`] when you want
    /// the same inset value for all sides.
    ///
    /// # Parameters
    ///
    /// * `value`: The value to be used for all sides.
    ///
    /// # Returns
    ///
    /// Returns a new [`EdgeInsets`] instance with all sides set to `value`.
    ///
    /// # Example
    ///
    /// ```
    /// use clew::EdgeInsets;
    ///
    /// let insets = EdgeInsets::all(10.0);
    /// assert_eq!(insets.top, 10.0);
    /// assert_eq!(insets.left, 10.0);
    /// assert_eq!(insets.right, 10.0);
    /// assert_eq!(insets.bottom, 10.0);
    /// ```
    pub fn all(value: f64) -> Self {
        Self {
            top: value,
            left: value,
            right: value,
            bottom: value,
        }
    }

    /// Creates a new [`EdgeInsets`] with symmetric horizontal and vertical insets.
    ///
    /// This method allows you to create an [`EdgeInsets`] instance where the left and right insets
    /// are the same (horizontal), and the top and bottom insets are the same (vertical).
    ///
    /// # Parameters
    ///
    /// * `horizontal`: The value to be used for both left and right insets.
    /// * `vertical`: The value to be used for both top and bottom insets.
    ///
    /// # Returns
    ///
    /// Returns a new [`EdgeInsets`] instance with the specified symmetric insets.
    ///
    /// # Example
    ///
    /// ```
    /// use clew::EdgeInsets;
    ///
    /// let insets = EdgeInsets::symmetric(20.0, 10.0);
    /// assert_eq!(insets.left, 20.0);
    /// assert_eq!(insets.right, 20.0);
    /// assert_eq!(insets.top, 10.0);
    /// assert_eq!(insets.bottom, 10.0);
    /// ```
    pub fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self {
            top: vertical,
            left: horizontal,
            right: horizontal,
            bottom: vertical,
        }
    }

    /// Returns the sum of the left and right insets.
    ///
    /// This method is useful when you need to know the total horizontal inset.
    ///
    /// # Returns
    ///
    /// Returns the sum of `self.left` and `self.right`.
    ///
    /// # Example
    ///
    /// ```
    /// use clew::EdgeInsets;
    ///
    /// let insets = EdgeInsets {
    ///     top: 10.0,
    ///     left: 15.0,
    ///     right: 20.0,
    ///     bottom: 10.0
    /// };
    /// assert_eq!(insets.horizontal(), 35.0);
    /// ```
    pub fn horizontal(&self) -> f64 {
        self.left + self.right
    }

    /// Returns the sum of the top and bottom insets.
    ///
    /// This method is useful when you need to know the total vertical inset.
    ///
    /// # Returns
    ///
    /// Returns the sum of `self.top` and `self.bottom`.
    ///
    /// # Example
    ///
    /// ```
    /// use clew::EdgeInsets;
    ///
    /// let insets = EdgeInsets {
    ///     top: 15.0,
    ///     left: 10.0,
    ///     right: 10.0,
    ///     bottom: 25.0
    /// };
    /// assert_eq!(insets.vertical(), 40.0);
    /// ```
    pub fn vertical(&self) -> f64 {
        self.top + self.bottom
    }
}

impl Add<EdgeInsets> for EdgeInsets {
    type Output = Self;

    fn add(self, rhs: EdgeInsets) -> Self::Output {
        Self {
            top: self.top + rhs.top,
            left: self.left + rhs.left,
            right: self.right + rhs.right,
            bottom: self.bottom + rhs.bottom,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Constraints {
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: f64,
    pub max_height: f64,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min_width: 0.,
            min_height: 0.,
            max_width: f64::INFINITY,
            max_height: f64::INFINITY,
        }
    }
}

impl Constraints {
    pub fn expand(self, padding: EdgeInsets) -> Constraints {
        Constraints {
            min_width: self.min_width + padding.horizontal(),
            min_height: self.min_height + padding.vertical(),
            max_width: self.max_width + padding.horizontal(),
            max_height: self.max_height + padding.vertical(),
        }
    }

    pub fn exact_size(size: impl Into<Size>) -> Self {
        let size = size.into();

        let width = match size.width {
            SizeConstraint::Fill(_) => f64::INFINITY,
            SizeConstraint::Wrap => 0.,
            SizeConstraint::Fixed(value) => value,
        };

        let height = match size.height {
            SizeConstraint::Fill(_) => f64::INFINITY,
            SizeConstraint::Wrap => 0.,
            SizeConstraint::Fixed(value) => value,
        };

        Constraints {
            min_width: width,
            min_height: width,
            max_width: height,
            max_height: height,
        }
    }
}

pub trait Scalar:
    Copy
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
    + std::fmt::Debug
{
    const ZERO: Self;

    const TWO: Self;
}

impl Scalar for f32 {
    const ZERO: Self = 0.0;

    const TWO: Self = 2.0;
}

impl Scalar for f64 {
    const ZERO: Self = 0.0;

    const TWO: Self = 2.0;
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, align(16))]
pub struct Vec2<T: Scalar = f64> {
    pub x: T,
    pub y: T,
}

impl<T: Scalar> Default for Vec2<T> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<T: Scalar> Vec2<T> {
    pub const ZERO: Self = Vec2 {
        x: T::ZERO,
        y: T::ZERO,
    };

    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl Vec2<f32> {
    #[inline]
    pub fn as_f64(self) -> Vec2<f64> {
        Vec2 {
            x: self.x as f64,
            y: self.y as f64,
        }
    }
}

impl Vec2<f64> {
    #[inline]
    pub fn as_f32(self) -> Vec2<f32> {
        Vec2 {
            x: self.x as f32,
            y: self.y as f32,
        }
    }
}

impl<T: Scalar> Add for Vec2<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Scalar> Sub for Vec2<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: Scalar> Mul<T> for Vec2<T> {
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl<T: Scalar> Div<T> for Vec2<T> {
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl<T: Scalar> Neg for Vec2<T> {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl<T: Scalar> AddAssign for Vec2<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Scalar> SubAssign for Vec2<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: Scalar> MulAssign<T> for Vec2<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<T: Scalar> DivAssign<T> for Vec2<T> {
    fn div_assign(&mut self, rhs: T) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

#[derive(Debug, Default, Clone)]
pub struct PhysicalSize {
    pub width: u32,
    pub height: u32,
}

impl PhysicalSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub(crate) fn to_vec2(&self) -> Vec2 {
        Vec2::new(self.width as f64, self.height as f64)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewId(pub usize);

#[derive(Debug, Clone)]
pub struct View {
    pub id: ViewId,
    pub physical_size: PhysicalSize,
    pub scale_factor: f64,
    pub safe_area: EdgeInsets,
}

impl View {
    pub fn size(&self) -> Vec2 {
        self.physical_size.to_vec2() / self.scale_factor
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C, align(16))]
pub struct Rect<T: Scalar = f64> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T: Scalar> Default for Rect<T> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Rect<f32> {
    #[inline]
    pub fn as_f64(self) -> Rect<f64> {
        Rect {
            x: self.x as f64,
            y: self.y as f64,
            width: self.width as f64,
            height: self.height as f64,
        }
    }
}

impl Rect<f64> {
    #[inline]
    pub fn as_f32(self) -> Rect<f32> {
        Rect {
            x: self.x as f32,
            y: self.y as f32,
            width: self.width as f32,
            height: self.height as f32,
        }
    }
}

impl<T: Scalar> Mul<T> for Rect<T> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: T) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl<T: Scalar> Rect<T> {
    pub const ZERO: Self = Self {
        x: T::ZERO,
        y: T::ZERO,
        width: T::ZERO,
        height: T::ZERO,
    };

    #[inline]
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[inline]
    pub fn from_pos_size(pos: Vec2<T>, size: Vec2<T>) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            width: size.x,
            height: size.y,
        }
    }

    #[inline]
    pub fn inverse_y(&self) -> Self {
        Self {
            x: self.x,
            y: -self.y,
            width: self.width,
            height: self.height,
        }
    }

    #[inline]
    pub fn left(&self) -> T {
        self.x
    }

    #[inline]
    pub fn right(&self) -> T {
        self.x + self.width
    }

    #[inline]
    pub fn top(&self) -> T {
        self.y
    }

    #[inline]
    pub fn bottom(&self) -> T {
        self.y + self.height
    }

    #[inline]
    pub fn position(&self) -> Vec2<T> {
        Vec2::new(self.x, self.y)
    }

    #[inline]
    pub fn size(&self) -> Vec2<T> {
        Vec2::new(self.width, self.height)
    }

    #[inline]
    pub fn expand(&self, size: T) -> Self {
        Self {
            x: self.x - size,
            y: self.y - size,
            width: self.width + size * T::TWO,
            height: self.height + size * T::TWO,
        }
    }

    #[inline]
    pub fn shrink(&self, size: T) -> Self {
        self.expand(-size)
    }

    #[inline]
    pub fn offset(&self, dx: T, dy: T) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            width: self.width,
            height: self.height,
        }
    }
}

#[inline]
pub fn point_with_rect_hit_test<T: Scalar>(point: Vec2<T>, rect: Rect<T>) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

#[inline]
pub fn rect_contains_boundary<T: Scalar>(boundary: Rect<T>, rect: Rect<T>) -> bool {
    let pos = boundary.position();
    let size = boundary.size();

    point_with_rect_hit_test(pos, rect)
        || point_with_rect_hit_test(Vec2::new(pos.x + size.x, pos.y), rect)
        || point_with_rect_hit_test(Vec2::new(pos.x, pos.y + size.y), rect)
        || point_with_rect_hit_test(Vec2::new(pos.x + size.x, pos.y + size.y), rect)
}

#[inline]
pub fn rects_overlap<T: Scalar>(a: Rect<T>, b: Rect<T>) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct ColorRgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Default, Debug, Clone, PartialEq, Copy)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Default, Debug, Clone, PartialEq, Copy)]
pub struct ColorOkLab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

impl ColorRgb {
    pub fn new(r: f32, b: f32, g: f32) -> Self {
        ColorRgb { r, g, b }
    }

    pub fn with_alpha(&self, a: f32) -> ColorRgba {
        ColorRgba::new(self.r, self.b, self.g, a)
    }

    pub fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex & 0xFF0000) >> 16) as f32 / 255.,
            g: ((hex & 0x00FF00) >> 8) as f32 / 255.,
            b: (hex & 0x0000FF) as f32 / 255.,
        }
    }

    pub fn to_hex(&self) -> u32 {
        let r = (self.r * 255.) as u32;
        let g = (self.g * 255.) as u32;
        let b = (self.b * 255.) as u32;

        (r << 16) | (g << 8) | b
    }

    /// Source: https://bottosson.github.io/posts/oklab/
    pub fn to_oklab(&self) -> ColorOkLab {
        let r = self.r as f64;
        let g = self.g as f64;
        let b = self.b as f64;

        let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
        let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
        let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

        let l = l.cbrt();
        let m = m.cbrt();
        let s = s.cbrt();

        ColorOkLab {
            l: 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
            a: 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
            b: 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
        }
    }
}

impl ColorRgba {
    pub const TRANSPARENT: Self = Self {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 0.,
    };

    pub fn transparent() -> ColorRgba {
        ColorRgba::new(0., 0., 0., 0.)
    }

    pub fn new(r: f32, b: f32, g: f32, a: f32) -> Self {
        ColorRgba { r, g, b, a }
    }

    pub fn to_rgb(&self) -> ColorRgb {
        ColorRgb {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex & 0x00FF0000) >> 16) as f32 / 255.,
            g: ((hex & 0x0000FF00) >> 8) as f32 / 255.,
            b: (hex & 0x000000FF) as f32 / 255.,
            a: ((hex & 0xFF000000) >> 24) as f32 / 255.,
        }
    }

    pub fn to_hex(&self) -> u32 {
        let r = (self.r * 255.) as u32;
        let g = (self.g * 255.) as u32;
        let b = (self.b * 255.) as u32;
        let a = (self.a * 255.) as u32;

        (a << 24) | (r << 16) | (g << 8) | b
    }

    pub fn with_opacity(&self, opacity: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: opacity,
        }
    }
}

impl ColorOkLab {
    /// Source: https://bottosson.github.io/posts/oklab/
    pub fn to_rgb(&self) -> ColorRgb {
        let l = self.l + 0.3963377774 * self.a + 0.2158037573 * self.b;
        let m = self.l - 0.1055613458 * self.a - 0.0638541728 * self.b;
        let s = self.l - 0.0894841775 * self.a - 1.2914855480 * self.b;

        let l = l * l * l;
        let m = m * m * m;
        let s = s * s * s;

        ColorRgb {
            r: (4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s) as f32,
            g: (-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s) as f32,
            b: (-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s) as f32,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Copy)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}

#[derive(Default, Debug, Clone, PartialEq, Copy)]
pub struct Border {
    pub top: Option<BorderSide>,
    pub right: Option<BorderSide>,
    pub bottom: Option<BorderSide>,
    pub left: Option<BorderSide>,
}

#[derive(Default, Debug, Clone, PartialEq, Copy)]
pub struct BorderSide {
    pub width: f32,
    pub color: ColorRgba,
}

impl BorderRadius {
    pub const ZERO: Self = Self {
        top_left: 0.,
        top_right: 0.,
        bottom_left: 0.,
        bottom_right: 0.,
    };

    /// Creates a BorderRadius with individual values for each corner
    pub fn new(top_left: f32, top_right: f32, bottom_left: f32, bottom_right: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    /// Creates a BorderRadius with the same radius for all corners
    pub fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_left: radius,
            bottom_right: radius,
        }
    }

    /// Creates a BorderRadius with the same radius for top and bottom
    pub fn vertical(top: f32, bottom: f32) -> Self {
        Self {
            top_left: top,
            top_right: top,
            bottom_left: bottom,
            bottom_right: bottom,
        }
    }

    /// Creates a BorderRadius with the same radius for left and right
    pub fn horizontal(left: f32, right: f32) -> Self {
        Self {
            top_left: left,
            top_right: right,
            bottom_left: left,
            bottom_right: right,
        }
    }

    /// Creates a BorderRadius with radius only on top corners
    pub fn top(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_left: 0.0,
            bottom_right: 0.0,
        }
    }

    /// Creates a BorderRadius with radius only on bottom corners
    pub fn bottom(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_left: radius,
            bottom_right: radius,
        }
    }

    /// Creates a BorderRadius with radius only on left corners
    pub fn left(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: 0.0,
            bottom_left: radius,
            bottom_right: 0.0,
        }
    }

    /// Creates a BorderRadius with radius only on right corners
    pub fn right(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: radius,
            bottom_left: 0.0,
            bottom_right: radius,
        }
    }
}

impl Border {
    /// Creates a Border with individual sides
    pub fn new(
        top: Option<BorderSide>,
        right: Option<BorderSide>,
        bottom: Option<BorderSide>,
        left: Option<BorderSide>,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates a Border with symmetric horizontal (left/right) and vertical (top/bottom) sides
    pub fn symmetric(horizontal: BorderSide, vertical: BorderSide) -> Self {
        Self {
            top: Some(vertical),
            right: Some(horizontal),
            bottom: Some(vertical),
            left: Some(horizontal),
        }
    }

    /// Creates a Border with the same side applied to all edges
    pub fn all(side: BorderSide) -> Self {
        Self {
            top: Some(side),
            right: Some(side),
            bottom: Some(side),
            left: Some(side),
        }
    }

    /// Creates a Border with only horizontal sides (left and right)
    pub fn horizontal(side: BorderSide) -> Self {
        Self {
            top: None,
            right: Some(side),
            bottom: None,
            left: Some(side),
        }
    }

    /// Creates a Border with only vertical sides (top and bottom)
    pub fn vertical(side: BorderSide) -> Self {
        Self {
            top: Some(side),
            right: None,
            bottom: Some(side),
            left: None,
        }
    }

    /// Creates a Border with only a top side
    pub fn top(side: BorderSide) -> Self {
        Self {
            top: Some(side),
            right: None,
            bottom: None,
            left: None,
        }
    }

    /// Creates a Border with only a bottom side
    pub fn bottom(side: BorderSide) -> Self {
        Self {
            top: None,
            right: None,
            bottom: Some(side),
            left: None,
        }
    }

    /// Creates a Border with only a left side
    pub fn left(side: BorderSide) -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: Some(side),
        }
    }

    /// Creates a Border with only a right side
    pub fn right(side: BorderSide) -> Self {
        Self {
            top: None,
            right: Some(side),
            bottom: None,
            left: None,
        }
    }
}

impl BorderSide {
    /// Creates a new BorderSide
    pub fn new(width: f32, color: ColorRgba) -> Self {
        Self { width, color }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Gradient {
    Linear(LinearGradient),
    Radial(RadialGradient),
    Sweep(SweepGradient),
}

// #[derive(Debug, Clone, PartialEq)]
// pub struct LinearGradient {
//     /// Start point (normalized 0.0 to 1.0)
//     pub start: (f32, f32),
//     /// End point (normalized 0.0 to 1.0)
//     pub end: (f32, f32),
//     /// Color stops
//     pub stops: Vec<ColorStop>,
//     /// How to handle colors outside the gradient range
//     pub tile_mode: TileMode,
// }

#[derive(Debug, Clone, PartialEq)]
pub struct RadialGradient {
    /// Center point (normalized 0.0 to 1.0)
    pub center: (f32, f32),
    /// Radius (normalized, typically 0.0 to 1.0)
    pub radius: f32,
    /// Optional focal point for elliptical gradients
    pub focal: Option<(f32, f32)>,
    /// Optional focal radius
    pub focal_radius: Option<f32>,
    /// Color stops
    pub stops: Vec<ColorStop>,
    /// How to handle colors outside the gradient range
    pub tile_mode: TileMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SweepGradient {
    /// Center point (normalized 0.0 to 1.0)
    pub center: (f32, f32),
    /// Start angle in radians (0 = right, π/2 = down)
    pub start_angle: f32,
    /// End angle in radians
    pub end_angle: f32,
    /// Color stops
    pub stops: Vec<ColorStop>,
    /// How to handle colors outside the gradient range
    pub tile_mode: TileMode,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct ColorStop {
    /// Position along the gradient (0.0 to 1.0)
    pub offset: f32,
    /// Color at this position
    pub color: ColorRgba,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum TileMode {
    /// Clamp to edge colors
    Clamp,
    /// Repeat the gradient
    Repeat,
    /// Repeat the gradient in reverse (mirror)
    Mirror,
    /// Decal - render transparent outside gradient
    Decal,
}

// impl LinearGradient {
//     /// Creates a simple top-to-bottom gradient
//     pub fn vertical(colors: Vec<ColorRgba>) -> Self {
//         Self {
//             start: (0.5, 0.0),
//             end: (0.5, 1.0),
//             stops: Self::even_stops(colors),
//             tile_mode: TileMode::Clamp,
//         }
//     }

//     /// Creates a simple left-to-right gradient
//     pub fn horizontal(colors: Vec<ColorRgba>) -> Self {
//         Self {
//             start: (0.0, 0.5),
//             end: (1.0, 0.5),
//             stops: Self::even_stops(colors),
//             tile_mode: TileMode::Clamp,
//         }
//     }

//     /// Creates a gradient at a specific angle (in radians)
//     pub fn angled(angle: f32, colors: Vec<ColorRgba>) -> Self {
//         let (dx, dy) = (angle.cos(), angle.sin());
//         Self {
//             start: (0.5 - dx * 0.5, 0.5 - dy * 0.5),
//             end: (0.5 + dx * 0.5, 0.5 + dy * 0.5),
//             stops: Self::even_stops(colors),
//             tile_mode: TileMode::Clamp,
//         }
//     }

//     pub fn new(start: (f32, f32), end: (f32, f32), stops: Vec<ColorStop>) -> Self {
//         Self {
//             start,
//             end,
//             stops,
//             tile_mode: TileMode::Clamp,
//         }
//     }

//     fn even_stops(colors: Vec<ColorRgba>) -> Vec<ColorStop> {
//         let count = colors.len();
//         if count == 0 {
//             return vec![];
//         }
//         colors
//             .into_iter()
//             .enumerate()
//             .map(|(i, color)| ColorStop {
//                 offset: i as f32 / (count - 1).max(1) as f32,
//                 color,
//             })
//             .collect()
//     }
// }
/// Most gradients have 2-4 stops, so inline up to 4
pub type ColorStops = SmallVec<[ColorStop; 4]>;

#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub stops: ColorStops,
    pub tile_mode: TileMode,
}

impl LinearGradient {
    pub fn vertical(colors: impl IntoColorStops) -> Self {
        Self {
            start: (0.5, 0.0),
            end: (0.5, 1.0),
            stops: colors.into_even_stops(),
            tile_mode: TileMode::Clamp,
        }
    }

    pub fn horizontal(colors: impl IntoColorStops) -> Self {
        Self {
            start: (0.0, 0.5),
            end: (1.0, 0.5),
            stops: colors.into_even_stops(),
            tile_mode: TileMode::Clamp,
        }
    }

    pub fn angled(angle: f32, colors: impl IntoColorStops) -> Self {
        let (dx, dy) = (angle.cos(), angle.sin());
        Self {
            start: (0.5 - dx * 0.5, 0.5 - dy * 0.5),
            end: (0.5 + dx * 0.5, 0.5 + dy * 0.5),
            stops: colors.into_even_stops(),
            tile_mode: TileMode::Clamp,
        }
    }

    pub fn new(start: (f32, f32), end: (f32, f32), stops: impl Into<ColorStops>) -> Self {
        Self {
            start,
            end,
            stops: stops.into(),
            tile_mode: TileMode::Clamp,
        }
    }

    pub fn with_tile_mode(mut self, mode: TileMode) -> Self {
        self.tile_mode = mode;
        self
    }

    fn even_stops(colors: Vec<ColorRgba>) -> Vec<ColorStop> {
        let count = colors.len();
        if count == 0 {
            return vec![];
        }
        colors
            .into_iter()
            .enumerate()
            .map(|(i, color)| ColorStop {
                offset: i as f32 / (count - 1).max(1) as f32,
                color,
            })
            .collect()
    }
}

/// Trait for types that can be converted into evenly-spaced color stops
pub trait IntoColorStops {
    fn into_even_stops(self) -> ColorStops;
}

// Array implementations - zero allocation for common cases
impl<const N: usize> IntoColorStops for [ColorRgba; N] {
    fn into_even_stops(self) -> ColorStops {
        even_stops_from_iter(self.into_iter(), N)
    }
}

impl IntoColorStops for (ColorRgba, ColorRgba) {
    fn into_even_stops(self) -> ColorStops {
        smallvec![
            ColorStop {
                offset: 0.0,
                color: self.0
            },
            ColorStop {
                offset: 1.0,
                color: self.1
            },
        ]
    }
}

impl IntoColorStops for (ColorRgba, ColorRgba, ColorRgba) {
    fn into_even_stops(self) -> ColorStops {
        smallvec![
            ColorStop {
                offset: 0.0,
                color: self.0
            },
            ColorStop {
                offset: 0.5,
                color: self.1
            },
            ColorStop {
                offset: 1.0,
                color: self.2
            },
        ]
    }
}

// Vec fallback for dynamic cases
impl IntoColorStops for Vec<ColorRgba> {
    fn into_even_stops(self) -> ColorStops {
        let len = self.len();
        even_stops_from_iter(self.into_iter(), len)
    }
}

// Direct ColorStops passthrough
impl IntoColorStops for ColorStops {
    fn into_even_stops(self) -> ColorStops {
        self
    }
}

fn even_stops_from_iter(iter: impl Iterator<Item = ColorRgba>, count: usize) -> ColorStops {
    if count == 0 {
        return ColorStops::new();
    }

    let divisor = (count - 1).max(1) as f32;

    let mut stops = ColorStops::with_capacity(count);

    for (i, color) in iter.enumerate() {
        stops.push(ColorStop {
            offset: i as f32 / divisor,
            color,
        });
    }

    stops
}
impl RadialGradient {
    /// Creates a simple radial gradient from center
    pub fn circle(colors: Vec<ColorRgba>) -> Self {
        Self {
            center: (0.5, 0.5),
            radius: 0.5,
            focal: None,
            focal_radius: None,
            stops: LinearGradient::even_stops(colors),
            tile_mode: TileMode::Clamp,
        }
    }

    pub fn new(center: (f32, f32), radius: f32, stops: Vec<ColorStop>) -> Self {
        Self {
            center,
            radius,
            focal: None,
            focal_radius: None,
            stops,
            tile_mode: TileMode::Clamp,
        }
    }
}

impl SweepGradient {
    /// Creates a full 360° sweep gradient
    pub fn full(colors: Vec<ColorRgba>) -> Self {
        Self {
            center: (0.5, 0.5),
            start_angle: 0.0,
            end_angle: std::f32::consts::TAU, // 2π
            stops: LinearGradient::even_stops(colors),
            tile_mode: TileMode::Clamp,
        }
    }

    pub fn new(
        center: (f32, f32),
        start_angle: f32,
        end_angle: f32,
        stops: Vec<ColorStop>,
    ) -> Self {
        Self {
            center,
            start_angle,
            end_angle,
            stops,
            tile_mode: TileMode::Clamp,
        }
    }
}

impl ColorStop {
    pub fn new(offset: f32, color: ColorRgba) -> Self {
        Self { offset, color }
    }
}

impl From<cosmic_text::Color> for ColorRgba {
    fn from(value: cosmic_text::Color) -> Self {
        Self {
            r: value.r() as f32 / 255.0,
            g: value.g() as f32 / 255.0,
            b: value.b() as f32 / 255.0,
            a: value.a() as f32 / 255.0,
        }
    }
}

impl From<ColorRgba> for cosmic_text::Color {
    fn from(value: ColorRgba) -> Self {
        Self::rgba(
            (value.r * 255.).clamp(0., 255.) as u8,
            (value.g * 255.).clamp(0., 255.) as u8,
            (value.b * 255.).clamp(0., 255.) as u8,
            (value.a * 255.).clamp(0., 255.) as u8,
        )
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum BoxShape {
    Rect,
    Oval,
}
