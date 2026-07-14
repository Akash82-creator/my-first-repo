/**
 * Equation Solver - Linear, quadratic, cubic, quartic, and systems
 */

class EquationSolver {
    // Linear: ax + b = 0
    static linear(a, b) {
        if (a === 0) {
            return b === 0 ? { type: 'infinite' } : { type: 'none' };
        }
        return {
            type: 'single',
            solutions: [-b / a],
            exact: [`x = ${-b}/${a}`]
        };
    }

    // Quadratic: ax² + bx + c = 0
    static quadratic(a, b, c) {
        if (a === 0) {
            return EquationSolver.linear(b, c);
        }
        
        const discriminant = b * b - 4 * a * c;
        
        if (discriminant > 0) {
            const x1 = (-b + Math.sqrt(discriminant)) / (2 * a);
            const x2 = (-b - Math.sqrt(discriminant)) / (2 * a);
            return {
                type: 'two_real',
                solutions: [x1, x2],
                discriminant,
                exact: [`x = (${b >= 0 ? '-' : ''}${Math.abs(b)} ± √${discriminant}) / ${2*a}`]
            };
        } else if (discriminant === 0) {
            const x = -b / (2 * a);
            return {
                type: 'one_real',
                solutions: [x],
                discriminant,
                exact: [`x = ${-b} / ${2*a}`]
            };
        } else {
            const real = -b / (2 * a);
            const imag = Math.sqrt(-discriminant) / (2 * a);
            return {
                type: 'complex',
                solutions: [{ re: real, im: imag }, { re: real, im: -imag }],
                discriminant,
                exact: [`x = ${real} ± ${imag}i`]
            };
        }
    }

    // Cubic: ax³ + bx² + cx + d = 0
    static cubic(a, b, c, d) {
        if (a === 0) {
            return EquationSolver.quadratic(b, c, d);
        }
        
        // Normalize
        b /= a; c /= a; d /= a;
        
        const p = c - b * b / 3;
        const q = 2 * b * b * b / 27 - b * c / 3 + d;
        
        const discriminant = q * q / 4 + p * p * p / 27;
        
        if (discriminant > 0) {
            // One real root
            const sqrtDisc = Math.sqrt(discriminant);
            const u = Math.cbrt(-q / 2 + sqrtDisc);
            const v = Math.cbrt(-q / 2 - sqrtDisc);
            const x1 = u + v - b / 3;
            
            const real = -(u + v) / 2 - b / 3;
            const imag = (u - v) * Math.sqrt(3) / 2;
            
            return {
                type: 'one_real_two_complex',
                solutions: [x1, { re: real, im: imag }, { re: real, im: -imag }],
                discriminant
            };
        } else if (discriminant === 0) {
            // Multiple roots
            if (p === 0 && q === 0) {
                return {
                    type: 'triple',
                    solutions: [-b / 3, -b / 3, -b / 3],
                    discriminant
                };
            }
            const u = Math.cbrt(-q / 2);
            return {
                type: 'double_root',
                solutions: [2 * u - b / 3, -u - b / 3],
                discriminant
            };
        } else {
            // Three real roots (casus irreducibilis)
            const r = Math.sqrt(-p * p * p / 27);
            const theta = Math.acos(-q / (2 * r));
            const rho = Math.cbrt(r);
            
            const x1 = 2 * rho * Math.cos(theta / 3) - b / 3;
            const x2 = 2 * rho * Math.cos((theta + 2 * Math.PI) / 3) - b / 3;
            const x3 = 2 * rho * Math.cos((theta + 4 * Math.PI) / 3) - b / 3;
            
            return {
                type: 'three_real',
                solutions: [x1, x2, x3],
                discriminant
            };
        }
    }

    // Quartic: ax⁴ + bx³ + cx² + dx + e = 0
    static quartic(a, b, c, d, e) {
        if (a === 0) {
            return EquationSolver.cubic(b, c, d, e);
        }
        
        // Ferrari's method (simplified)
        b /= a; c /= a; d /= a; e /= a;
        
        const p = c - 3 * b * b / 8;
        const q = d - b * c / 2 + b * b * b / 8;
        const r = e - b * d / 4 + b * b * c / 16 - 3 * b * b * b * b / 256;
        
        if (q === 0) {
            // Biquadratic case
            return EquationSolver.biquadratic(1, p, r);
        }
        
        // Solve resolvent cubic
        const cubicResult = EquationSolver.cubic(1, p / 2, (p * p - 4 * r) / 16, -q * q / 64);
        const y = cubicResult.solutions.find(s => typeof s === 'number' && s > 0) || 0;
        
        const sqrtY = Math.sqrt(y);
        const sqrtPPlus2Y = Math.sqrt(p + 2 * y);
        
        const x1 = (-sqrtY + sqrtPPlus2Y) / 2 - b / 4;
        const x2 = (-sqrtY - sqrtPPlus2Y) / 2 - b / 4;
        const x3 = (sqrtY + Math.sqrt(p + 2 * y - 2 * q / sqrtY)) / 2 - b / 4;
        const x4 = (sqrtY - Math.sqrt(p + 2 * y - 2 * q / sqrtY)) / 2 - b / 4;
        
        return {
            type: 'four_roots',
            solutions: [x1, x2, x3, x4]
        };
    }

    // Biquadratic: ax⁴ + bx² + c = 0
    static biquadratic(a, b, c) {
        if (a === 0) {
            return EquationSolver.quadratic(b, 0, c);
        }
        
        // Let y = x²
        const quadResult = EquationSolver.quadratic(a, b, c);
        const solutions = [];
        
        for (const y of quadResult.solutions) {
            if (typeof y === 'number') {
                if (y >= 0) {
                    solutions.push(Math.sqrt(y), -Math.sqrt(y));
                } else {
                    solutions.push({ re: 0, im: Math.sqrt(-y) }, { re: 0, im: -Math.sqrt(-y) });
                }
            }
        }
        
        return { type: 'biquadratic', solutions };
    }

    // System of 2 linear equations
    // a1*x + b1*y = c1
    // a2*x + b2*y = c2
    static system2x2(a1, b1, c1, a2, b2, c2) {
        const det = a1 * b2 - a2 * b1;
        
        if (det === 0) {
            // Check if parallel or coincident
            if (a1 * c2 === a2 * c1 && b1 * c2 === b2 * c1) {
                return { type: 'infinite' };
            }
            return { type: 'none' };
        }
        
        const x = (c1 * b2 - c2 * b1) / det;
        const y = (a1 * c2 - a2 * c1) / det;
        
        return {
            type: 'unique',
            solutions: { x, y }
        };
    }

    // System of 3 linear equations using Cramer's rule
    static system3x3(coeffs, constants) {
        const detA = EquationSolver.det3x3(coeffs);
        
        if (Math.abs(detA) < 1e-10) {
            return { type: 'no_unique_solution' };
        }
        
        const solutions = [];
        for (let i = 0; i < 3; i++) {
            const modified = coeffs.map(row => [...row]);
            for (let j = 0; j < 3; j++) {
                modified[j][i] = constants[j];
            }
            solutions.push(EquationSolver.det3x3(modified) / detA);
        }
        
        return {
            type: 'unique',
            solutions
        };
    }

    static det3x3(m) {
        return m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) -
               m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) +
               m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    }

    // Nonlinear equation using Newton-Raphson
    static nonlinear(func, deriv, x0, tolerance = 1e-10, maxIter = 100) {
        let x = x0;
        
        for (let i = 0; i < maxIter; i++) {
            const fx = func(x);
            const dfx = deriv ? deriv(x) : EquationSolver.numericalDeriv(func, x);
            
            if (Math.abs(dfx) < 1e-15) break;
            
            const xNew = x - fx / dfx;
            if (Math.abs(xNew - x) < tolerance) {
                return { root: xNew, iterations: i + 1, converged: true };
            }
            x = xNew;
        }
        
        return { root: x, iterations: maxIter, converged: false };
    }

    static numericalDeriv(func, x, h = 1e-8) {
        return (func(x + h) - func(x - h)) / (2 * h);
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = EquationSolver;
}
