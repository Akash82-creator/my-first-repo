/**
 * Calculus - Numerical differentiation and integration
 */

class Calculus {
    // Numerical differentiation using central difference
    static derivative(func, x, h = 1e-8) {
        return (func(x + h) - func(x - h)) / (2 * h);
    }

    // Second derivative
    static secondDerivative(func, x, h = 1e-5) {
        return (func(x + h) - 2 * func(x) + func(x - h)) / (h * h);
    }

    // Nth derivative (recursive)
    static nthDerivative(func, x, n, h = 1e-5) {
        if (n === 1) return Calculus.derivative(func, x, h);
        return Calculus.derivative((t) => Calculus.nthDerivative(func, t, n - 1, h), x, h);
    }

    // Trapezoidal rule for integration
    static trapezoidal(func, a, b, n = 1000) {
        const h = (b - a) / n;
        let sum = (func(a) + func(b)) / 2;
        
        for (let i = 1; i < n; i++) {
            sum += func(a + i * h);
        }
        
        return sum * h;
    }

    // Simpson's rule for better accuracy
    static simpson(func, a, b, n = 1000) {
        if (n % 2 === 1) n++;
        const h = (b - a) / n;
        
        let sum = func(a) + func(b);
        
        for (let i = 1; i < n; i++) {
            const x = a + i * h;
            sum += (i % 2 === 0 ? 2 : 4) * func(x);
        }
        
        return sum * h / 3;
    }

    // Adaptive Simpson's rule
    static adaptiveSimpson(func, a, b, tolerance = 1e-10) {
        function recurse(f, a, b, eps, whole) {
            const c = (a + b) / 2;
            const left = Calculus.simpson(f, a, c, 2);
            const right = Calculus.simpson(f, c, b, 2);
            const delta = left + right - whole;
            
            if (Math.abs(delta) <= 15 * eps) {
                return left + right + delta / 15;
            }
            return recurse(f, a, c, eps / 2, left) + recurse(f, c, b, eps / 2, right);
        }
        
        const whole = Calculus.simpson(func, a, b, 2);
        return recurse(func, a, b, tolerance, whole);
    }

    // Romberg integration
    static romberg(func, a, b, maxIter = 10) {
        const R = Array(maxIter).fill(null).map(() => Array(maxIter).fill(0));
        
        R[0][0] = Calculus.trapezoidal(func, a, b, 1);
        
        for (let i = 1; i < maxIter; i++) {
            const n = Math.pow(2, i);
            R[i][0] = Calculus.trapezoidal(func, a, b, n);
            
            for (let j = 1; j <= i; j++) {
                const factor = Math.pow(4, j);
                R[i][j] = (factor * R[i][j-1] - R[i-1][j-1]) / (factor - 1);
            }
        }
        
        return R[maxIter - 1][maxIter - 1];
    }

    // Find tangent line at point
    static tangentLine(func, x) {
        const y = func(x);
        const slope = Calculus.derivative(func, x);
        
        return {
            slope,
            intercept: y - slope * x,
            equation: `y = ${slope.toFixed(6)}x + ${(y - slope * x).toFixed(6)}`
        };
    }

    // Newton-Raphson root finding
    static newtonRaphson(func, x0, tolerance = 1e-10, maxIter = 100) {
        let x = x0;
        
        for (let i = 0; i < maxIter; i++) {
            const fx = func(x);
            const fpx = Calculus.derivative(func, x);
            
            if (Math.abs(fpx) < 1e-15) {
                throw new Error('Derivative too small');
            }
            
            const xNew = x - fx / fpx;
            
            if (Math.abs(xNew - x) < tolerance) {
                return { root: xNew, iterations: i + 1, converged: true };
            }
            
            x = xNew;
        }
        
        return { root: x, iterations: maxIter, converged: false };
    }

    // Secant method (doesn't require derivative)
    static secant(func, x0, x1, tolerance = 1e-10, maxIter = 100) {
        for (let i = 0; i < maxIter; i++) {
            const f0 = func(x0);
            const f1 = func(x1);
            
            if (Math.abs(f1 - f0) < 1e-15) {
                throw new Error('Function values too close');
            }
            
            const xNew = x1 - f1 * (x1 - x0) / (f1 - f0);
            
            if (Math.abs(xNew - x1) < tolerance) {
                return { root: xNew, iterations: i + 1, converged: true };
            }
            
            x0 = x1;
            x1 = xNew;
        }
        
        return { root: x1, iterations: maxIter, converged: false };
    }

    // Bisection method
    static bisection(func, a, b, tolerance = 1e-10, maxIter = 100) {
        if (func(a) * func(b) > 0) {
            throw new Error('Function must have opposite signs at endpoints');
        }
        
        for (let i = 0; i < maxIter; i++) {
            const c = (a + b) / 2;
            const fc = func(c);
            
            if (Math.abs(fc) < tolerance || (b - a) / 2 < tolerance) {
                return { root: c, iterations: i + 1, converged: true };
            }
            
            if (func(a) * fc < 0) {
                b = c;
            } else {
                a = c;
            }
        }
        
        return { root: (a + b) / 2, iterations: maxIter, converged: false };
    }

    // Fixed point iteration
    static fixedPoint(g, x0, tolerance = 1e-10, maxIter = 100) {
        let x = x0;
        
        for (let i = 0; i < maxIter; i++) {
            const xNew = g(x);
            
            if (Math.abs(xNew - x) < tolerance) {
                return { root: xNew, iterations: i + 1, converged: true };
            }
            
            x = xNew;
        }
        
        return { root: x, iterations: maxIter, converged: false };
    }

    // Arc length of curve y = f(x) from a to b
    static arcLength(func, a, b, n = 1000) {
        const integrand = (x) => Math.sqrt(1 + Calculus.derivative(func, x) ** 2);
        return Calculus.simpson(integrand, a, b, n);
    }

    // Area between two curves
    static areaBetween(f, g, a, b, n = 1000) {
        const integrand = (x) => Math.abs(f(x) - g(x));
        return Calculus.simpson(integrand, a, b, n);
    }

    // Volume of revolution (around x-axis)
    static volumeOfRevolution(func, a, b, n = 1000) {
        const integrand = (x) => Math.PI * func(x) ** 2;
        return Calculus.simpson(integrand, a, b, n);
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = Calculus;
}
