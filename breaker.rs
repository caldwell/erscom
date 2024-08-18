// Copyright © 2024 David Caldwell <david@porkrind.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

pub struct Empty; // () except we own it so we can create traits for it
pub struct Breaker<T>(std::ops::ControlFlow<Empty,T>); // Wrap ControlFlow for the same reason

impl<T> Breaker<T> {
    pub fn brk() -> Self { Breaker(std::ops::ControlFlow::Break(Empty)) }
    pub fn cont(t: T) -> Self { Breaker(std::ops::ControlFlow::Continue(t)) }
}

// fn x() { y()? }
impl std::ops::FromResidual<Empty> for () {
    fn from_residual(_residual: Empty) -> Self {
        ()
    }
}
// fn x() -> bool { y()?; true }
impl std::ops::FromResidual<Empty> for bool {
    fn from_residual(_residual: Empty) -> Self {
        false
    }
}
impl<T> std::ops::FromResidual<Empty> for Breaker<T> {
    fn from_residual(residual: Empty) -> Self {
        Breaker(std::ops::ControlFlow::Break(residual))
    }
}
impl<T> std::ops::Try for Breaker<T> {
    type Output = T;
    type Residual = Empty;
    fn from_output(output: Self::Output) -> Self {
        Breaker(std::ops::ControlFlow::Continue(output))
    }
    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        self.0
    }
}
