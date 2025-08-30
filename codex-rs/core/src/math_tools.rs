use anyhow::Result;
use std::f64;

/// Evaluate a mathematical expression supporting basic operators and common functions.
/// Uses the `meval` crate for parsing and evaluation.
pub(crate) fn evaluate_expression(expr: &str) -> Result<f64> {
    let parsed = expr.parse::<meval::Expr>()?;
    let mut ctx = meval::Context::new();
    ctx.var("tau", 2.0 * f64::consts::PI);
    ctx.var("pi", f64::consts::PI);
    ctx.var("e", f64::consts::E);
    let value = parsed.eval_with_context(ctx)?;
    Ok(value)
}

/// Solve ax^2 + bx + c = 0 and return the roots as strings.
pub(crate) fn solve_quadratic(a: f64, b: f64, c: f64) -> Vec<String> {
    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0 {
        let sqrt_disc = discriminant.sqrt();
        let x1 = (-b + sqrt_disc) / (2.0 * a);
        let x2 = (-b - sqrt_disc) / (2.0 * a);
        vec![x1.to_string(), x2.to_string()]
    } else {
        let sqrt_disc = (-discriminant).sqrt();
        let real = -b / (2.0 * a);
        let imag = sqrt_disc / (2.0 * a);
        vec![format!("{}+{}i", real, imag), format!("{}-{}i", real, imag)]
    }
}

/// Compute determinant of a 2x2 or 3x3 matrix.
pub(crate) fn matrix_determinant(matrix: &[Vec<f64>]) -> Result<f64> {
    let n = matrix.len();
    if n == 2 {
        Ok(matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0])
    } else if n == 3 {
        let a = matrix[0][0];
        let b = matrix[0][1];
        let c = matrix[0][2];
        let d = matrix[1][0];
        let e = matrix[1][1];
        let f = matrix[1][2];
        let g = matrix[2][0];
        let h = matrix[2][1];
        let i = matrix[2][2];
        Ok(a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g))
    } else {
        Err(anyhow::anyhow!("only 2x2 or 3x3 matrices are supported"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_expression() {
        let val = evaluate_expression("2 + 2 * 2").unwrap();
        assert_eq!(val, 6.0);
    }

    #[test]
    fn test_quadratic_real() {
        let roots = solve_quadratic(1.0, -3.0, 2.0);
        assert_eq!(roots, vec!["2".to_string(), "1".to_string()]);
    }

    #[test]
    fn test_matrix_det() {
        let det = matrix_determinant(&vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
        assert_eq!(det, -2.0);
    }
}
