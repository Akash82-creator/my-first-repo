/**
 * Statistics Calculator - Single and two-variable statistics
 */

class Statistics {
    constructor() {
        this.data = [];
    }

    setData(data) {
        if (Array.isArray(data)) {
            this.data = data;
        } else if (typeof data === 'string') {
            // Parse comma or newline separated values
            this.data = data.split(/[\n,]+/).map(s => parseFloat(s.trim())).filter(n => !isNaN(n));
        }
        return this.data;
    }

    // Single variable statistics
    count() {
        return this.data.length;
    }

    sum() {
        return this.data.reduce((a, b) => a + b, 0);
    }

    mean() {
        if (this.data.length === 0) return 0;
        return this.sum() / this.data.length;
    }

    median() {
        if (this.data.length === 0) return 0;
        const sorted = [...this.data].sort((a, b) => a - b);
        const mid = Math.floor(sorted.length / 2);
        if (sorted.length % 2 === 0) {
            return (sorted[mid - 1] + sorted[mid]) / 2;
        }
        return sorted[mid];
    }

    mode() {
        if (this.data.length === 0) return [];
        const freq = {};
        let maxFreq = 0;
        
        for (const val of this.data) {
            freq[val] = (freq[val] || 0) + 1;
            maxFreq = Math.max(maxFreq, freq[val]);
        }
        
        return Object.keys(freq)
            .filter(k => freq[k] === maxFreq)
            .map(Number);
    }

    variance(sample = true) {
        if (this.data.length < 2) return 0;
        const mean = this.mean();
        const sumSquaredDiff = this.data.reduce((sum, val) => sum + (val - mean) ** 2, 0);
        return sumSquaredDiff / (sample ? this.data.length - 1 : this.data.length);
    }

    standardDeviation(sample = true) {
        return Math.sqrt(this.variance(sample));
    }

    quartiles() {
        if (this.data.length === 0) return { Q1: 0, Q2: 0, Q3: 0 };
        const sorted = [...this.data].sort((a, b) => a - b);
        
        const percentile = (p) => {
            const index = (p / 100) * (sorted.length - 1);
            const lower = Math.floor(index);
            const upper = Math.ceil(index);
            if (lower === upper) return sorted[lower];
            return sorted[lower] * (upper - index) + sorted[upper] * (index - lower);
        };
        
        return {
            Q1: percentile(25),
            Q2: percentile(50), // Same as median
            Q3: percentile(75)
        };
    }

    iqr() {
        const q = this.quartiles();
        return q.Q3 - q.Q1;
    }

    min() {
        return this.data.length === 0 ? 0 : Math.min(...this.data);
    }

    max() {
        return this.data.length === 0 ? 0 : Math.max(...this.data);
    }

    range() {
        return this.max() - this.min();
    }

    // Two-variable statistics
    static correlation(x, y) {
        if (x.length !== y.length || x.length === 0) return 0;
        const n = x.length;
        const meanX = x.reduce((a, b) => a + b, 0) / n;
        const meanY = y.reduce((a, b) => a + b, 0) / n;
        
        let numerator = 0;
        let sumSqX = 0;
        let sumSqY = 0;
        
        for (let i = 0; i < n; i++) {
            const dx = x[i] - meanX;
            const dy = y[i] - meanY;
            numerator += dx * dy;
            sumSqX += dx * dx;
            sumSqY += dy * dy;
        }
        
        const denominator = Math.sqrt(sumSqX * sumSqY);
        return denominator === 0 ? 0 : numerator / denominator;
    }

    static covariance(x, y, sample = true) {
        if (x.length !== y.length || x.length === 0) return 0;
        const n = x.length;
        const meanX = x.reduce((a, b) => a + b, 0) / n;
        const meanY = y.reduce((a, b) => a + b, 0) / n;
        
        let sum = 0;
        for (let i = 0; i < n; i++) {
            sum += (x[i] - meanX) * (y[i] - meanY);
        }
        
        return sum / (sample ? n - 1 : n);
    }

    // Linear regression: y = ax + b
    static linearRegression(x, y) {
        if (x.length !== y.length || x.length === 0) {
            return { slope: 0, intercept: 0, rSquared: 0 };
        }
        
        const n = x.length;
        const sumX = x.reduce((a, b) => a + b, 0);
        const sumY = y.reduce((a, b) => a + b, 0);
        const sumXY = x.reduce((sum, xi, i) => sum + xi * y[i], 0);
        const sumX2 = x.reduce((sum, xi) => sum + xi * xi, 0);
        
        const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);
        const intercept = (sumY - slope * sumX) / n;
        
        const r = Statistics.correlation(x, y);
        
        return {
            slope,
            intercept,
            rSquared: r * r,
            equation: `y = ${slope.toFixed(4)}x + ${intercept.toFixed(4)}`
        };
    }

    // Polynomial regression (quadratic): y = ax² + bx + c
    static polynomialRegression(x, y, degree = 2) {
        const n = x.length;
        if (n <= degree) {
            return { coefficients: [], rSquared: 0 };
        }
        
        // Build the Vandermonde matrix and solve using normal equations
        const X = Array(n).fill(null).map((_, i) => 
            Array(degree + 1).fill(null).map((_, j) => Math.pow(x[i], j))
        );
        
        // X^T * X
        const XtX = Array(degree + 1).fill(null).map(() => Array(degree + 1).fill(0));
        for (let i = 0; i <= degree; i++) {
            for (let j = 0; j <= degree; j++) {
                for (let k = 0; k < n; k++) {
                    XtX[i][j] += X[k][i] * X[k][j];
                }
            }
        }
        
        // X^T * y
        const Xty = Array(degree + 1).fill(0);
        for (let i = 0; i <= degree; i++) {
            for (let k = 0; k < n; k++) {
                Xty[i] += X[k][i] * y[k];
            }
        }
        
        // Solve using Gaussian elimination
        const coeffs = Statistics.solveLinearSystem(XtX, Xty);
        
        // Calculate R²
        const meanY = y.reduce((a, b) => a + b, 0) / n;
        const ssTot = y.reduce((sum, yi) => sum + (yi - meanY) ** 2, 0);
        const ssRes = y.reduce((sum, yi, i) => {
            const pred = coeffs.reduce((acc, c, j) => acc + c * Math.pow(x[i], j), 0);
            return sum + (yi - pred) ** 2;
        }, 0);
        
        return {
            coefficients: coeffs,
            rSquared: 1 - ssRes / ssTot,
            equation: coeffs.map((c, i) => `${c.toFixed(4)}x^${i}`).join(' + ')
        };
    }

    static solveLinearSystem(A, b) {
        const n = A.length;
        const augmented = A.map((row, i) => [...row, b[i]]);
        
        // Forward elimination
        for (let col = 0; col < n; col++) {
            // Find pivot
            let maxRow = col;
            for (let row = col + 1; row < n; row++) {
                if (Math.abs(augmented[row][col]) > Math.abs(augmented[maxRow][col])) {
                    maxRow = row;
                }
            }
            [augmented[col], augmented[maxRow]] = [augmented[maxRow], augmented[col]];
            
            if (Math.abs(augmented[col][col]) < 1e-10) continue;
            
            // Eliminate column
            for (let row = col + 1; row < n; row++) {
                const factor = augmented[row][col] / augmented[col][col];
                for (let j = col; j <= n; j++) {
                    augmented[row][j] -= factor * augmented[col][j];
                }
            }
        }
        
        // Back substitution
        const x = Array(n).fill(0);
        for (let i = n - 1; i >= 0; i--) {
            let sum = augmented[i][n];
            for (let j = i + 1; j < n; j++) {
                sum -= augmented[i][j] * x[j];
            }
            x[i] = sum / (augmented[i][i] || 1);
        }
        
        return x;
    }

    // Get all single-variable statistics
    getAllStats() {
        return {
            count: this.count(),
            sum: this.sum(),
            mean: this.mean(),
            median: this.median(),
            mode: this.mode(),
            variance: this.variance(),
            stdDev: this.standardDeviation(),
            min: this.min(),
            max: this.max(),
            range: this.range(),
            quartiles: this.quartiles(),
            iqr: this.iqr()
        };
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = Statistics;
}
