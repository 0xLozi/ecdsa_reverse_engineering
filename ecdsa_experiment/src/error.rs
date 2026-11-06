#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `p` or `n` is zero.
    InvalidParameter,
    /// The base point is the point at infinity (0, 0).
    InfinityBasePoint,
    /// The base point does not satisfy the curve equation.
    PointNotOnCurve,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidParameter => write!(f, "Invalid curve parameter"),
            Error::InfinityBasePoint => write!(f, "base point is the point at infinity"),
            Error::PointNotOnCurve => write!(f, "base point is not on the curve"),
        }
    }
}


