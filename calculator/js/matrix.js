/**
 * Matrix Calculator - Supports matrices up to 10x10
 */

class Matrix {
    constructor(rows, cols, data = null) {
        this.rows = rows;
        this.cols = cols;
        if (data) {
            this.data = data;
        } else {
            this.data = Array(rows).fill(null).map(() => Array(cols).fill(0));
        }
    }

    static fromArray(arr) {
        const rows = arr.length;
        const cols = arr[0].length;
        return new Matrix(rows, cols, arr.map(row => [...row]));
    }

    get(row, col) {
        return this.data[row][col];
    }

    set(row, col, value) {
        this.data[row][col] = value;
    }

    add(other) {
        if (this.rows !== other.rows || this.cols !== other.cols) {
            throw new Error('Matrix dimensions must match for addition');
        }
        const result = new Matrix(this.rows, this.cols);
        for (let i = 0; i < this.rows; i++) {
            for (let j = 0; j < this.cols; j++) {
                result.data[i][j] = this.data[i][j] + other.data[i][j];
            }
        }
        return result;
    }

    subtract(other) {
        if (this.rows !== other.rows || this.cols !== other.cols) {
            throw new Error('Matrix dimensions must match for subtraction');
        }
        const result = new Matrix(this.rows, this.cols);
        for (let i = 0; i < this.rows; i++) {
            for (let j = 0; j < this.cols; j++) {
                result.data[i][j] = this.data[i][j] - other.data[i][j];
            }
        }
        return result;
    }

    multiply(other) {
        if (other instanceof Matrix) {
            if (this.cols !== other.rows) {
                throw new Error('Matrix dimensions incompatible for multiplication');
            }
            const result = new Matrix(this.rows, other.cols);
            for (let i = 0; i < this.rows; i++) {
                for (let j = 0; j < other.cols; j++) {
                    let sum = 0;
                    for (let k = 0; k < this.cols; k++) {
                        sum += this.data[i][k] * other.data[k][j];
                    }
                    result.data[i][j] = sum;
                }
            }
            return result;
        } else {
            // Scalar multiplication
            const result = new Matrix(this.rows, this.cols);
            for (let i = 0; i < this.rows; i++) {
                for (let j = 0; j < this.cols; j++) {
                    result.data[i][j] = this.data[i][j] * other;
                }
            }
            return result;
        }
    }

    transpose() {
        const result = new Matrix(this.cols, this.rows);
        for (let i = 0; i < this.rows; i++) {
            for (let j = 0; j < this.cols; j++) {
                result.data[j][i] = this.data[i][j];
            }
        }
        return result;
    }

    determinant() {
        if (this.rows !== this.cols) {
            throw new Error('Determinant only defined for square matrices');
        }
        
        if (this.rows === 1) return this.data[0][0];
        if (this.rows === 2) {
            return this.data[0][0] * this.data[1][1] - this.data[0][1] * this.data[1][0];
        }
        
        let det = 0;
        for (let j = 0; j < this.cols; j++) {
            const minor = this.getMinor(0, j);
            det += Math.pow(-1, j) * this.data[0][j] * minor.determinant();
        }
        return det;
    }

    getMinor(row, col) {
        const minor = new Matrix(this.rows - 1, this.cols - 1);
        for (let i = 0; i < this.rows; i++) {
            if (i === row) continue;
            for (let j = 0; j < this.cols; j++) {
                if (j === col) continue;
                const newRow = i < row ? i : i - 1;
                const newCol = j < col ? j : j - 1;
                minor.data[newRow][newCol] = this.data[i][j];
            }
        }
        return minor;
    }

    inverse() {
        if (this.rows !== this.cols) {
            throw new Error('Inverse only defined for square matrices');
        }
        
        const det = this.determinant();
        if (Math.abs(det) < 1e-10) {
            throw new Error('Matrix is singular and cannot be inverted');
        }
        
        if (this.rows === 1) {
            return new Matrix(1, 1, [[1 / this.data[0][0]]]);
        }
        
        if (this.rows === 2) {
            return new Matrix(2, 2, [
                [this.data[1][1] / det, -this.data[0][1] / det],
                [-this.data[1][0] / det, this.data[0][0] / det]
            ]);
        }
        
        // For larger matrices, use adjugate method
        const cofactor = new Matrix(this.rows, this.cols);
        for (let i = 0; i < this.rows; i++) {
            for (let j = 0; j < this.cols; j++) {
                const minor = this.getMinor(i, j);
                cofactor.data[i][j] = Math.pow(-1, i + j) * minor.determinant();
            }
        }
        
        const adjugate = cofactor.transpose();
        return adjugate.multiply(1 / det);
    }

    rank() {
        const rref = this.rowReduce();
        let rank = 0;
        for (let i = 0; i < rref.rows; i++) {
            if (rref.data[i].some(val => Math.abs(val) > 1e-10)) {
                rank++;
            }
        }
        return rank;
    }

    rowReduce() {
        const result = new Matrix(this.rows, this.cols, this.data.map(row => [...row]));
        let pivotRow = 0;
        
        for (let col = 0; col < result.cols && pivotRow < result.rows; col++) {
            // Find pivot
            let maxRow = pivotRow;
            for (let row = pivotRow + 1; row < result.rows; row++) {
                if (Math.abs(result.data[row][col]) > Math.abs(result.data[maxRow][col])) {
                    maxRow = row;
                }
            }
            
            if (Math.abs(result.data[maxRow][col]) < 1e-10) continue;
            
            // Swap rows
            [result.data[pivotRow], result.data[maxRow]] = [result.data[maxRow], result.data[pivotRow]];
            
            // Scale pivot row
            const pivot = result.data[pivotRow][col];
            for (let j = col; j < result.cols; j++) {
                result.data[pivotRow][j] /= pivot;
            }
            
            // Eliminate column
            for (let row = 0; row < result.rows; row++) {
                if (row !== pivotRow && Math.abs(result.data[row][col]) > 1e-10) {
                    const factor = result.data[row][col];
                    for (let j = col; j < result.cols; j++) {
                        result.data[row][j] -= factor * result.data[pivotRow][j];
                    }
                }
            }
            
            pivotRow++;
        }
        
        return result;
    }

    luDecomposition() {
        if (this.rows !== this.cols) {
            throw new Error('LU decomposition only defined for square matrices');
        }
        
        const n = this.rows;
        const L = new Matrix(n, n);
        const U = new Matrix(n, n, this.data.map(row => [...row]));
        
        for (let i = 0; i < n; i++) {
            L.data[i][i] = 1;
        }
        
        for (let k = 0; k < n; k++) {
            for (let i = k + 1; i < n; i++) {
                if (Math.abs(U.data[k][k]) < 1e-10) continue;
                const factor = U.data[i][k] / U.data[k][k];
                L.data[i][k] = factor;
                for (let j = k; j < n; j++) {
                    U.data[i][j] -= factor * U.data[k][j];
                }
            }
        }
        
        return { L, U };
    }

    eigenvalues() {
        if (this.rows !== this.cols) {
            throw new Error('Eigenvalues only defined for square matrices');
        }
        
        // Power iteration method for dominant eigenvalue
        const n = this.rows;
        const eigenvalues = [];
        const A = new Matrix(n, n, this.data.map(row => [...row]));
        
        for (let eig = 0; eig < n; eig++) {
            let v = Array(n).fill(1).map((_, i) => i === eig ? 1 : 0);
            let lambda = 0;
            
            for (let iter = 0; iter < 100; iter++) {
                // Multiply A * v
                const Av = Array(n).fill(0);
                for (let i = 0; i < n; i++) {
                    for (let j = 0; j < n; j++) {
                        Av[i] += A.data[i][j] * v[j];
                    }
                }
                
                // Find max component
                let maxVal = 0;
                let maxIdx = 0;
                for (let i = 0; i < n; i++) {
                    if (Math.abs(Av[i]) > Math.abs(maxVal)) {
                        maxVal = Av[i];
                        maxIdx = i;
                    }
                }
                
                const newLambda = maxVal;
                if (Math.abs(newLambda - lambda) < 1e-8) break;
                lambda = newLambda;
                
                // Normalize
                for (let i = 0; i < n; i++) {
                    v[i] = Av[i] / (maxVal || 1);
                }
            }
            
            eigenvalues.push(lambda);
            
            // Deflate matrix
            for (let i = 0; i < n; i++) {
                for (let j = 0; j < n; j++) {
                    A.data[i][j] -= lambda * v[i] * v[j];
                }
            }
        }
        
        return eigenvalues;
    }

    eigenvectors() {
        if (this.rows !== this.cols) {
            throw new Error('Eigenvectors only defined for square matrices');
        }
        
        const eigenvalues = this.eigenvalues();
        const n = this.rows;
        const eigenvectors = [];
        
        for (const lambda of eigenvalues) {
            // Solve (A - λI)v = 0
            const A = new Matrix(n, n);
            for (let i = 0; i < n; i++) {
                for (let j = 0; j < n; j++) {
                    A.data[i][j] = this.data[i][j] - (i === j ? lambda : 0);
                }
            }
            
            // Use power iteration to find eigenvector
            let v = Array(n).fill(1 / Math.sqrt(n));
            for (let iter = 0; iter < 100; iter++) {
                const Av = Array(n).fill(0);
                for (let i = 0; i < n; i++) {
                    for (let j = 0; j < n; j++) {
                        Av[i] += this.data[i][j] * v[j];
                    }
                }
                
                // Normalize
                const norm = Math.sqrt(Av.reduce((sum, val) => sum + val * val, 0));
                v = Av.map(val => val / (norm || 1));
            }
            
            eigenvectors.push(v);
        }
        
        return eigenvectors;
    }

    toString(precision = 4) {
        let str = '[';
        for (let i = 0; i < this.rows; i++) {
            if (i > 0) str += ' ';
            str += '[';
            for (let j = 0; j < this.cols; j++) {
                if (j > 0) str += ', ';
                str += this.data[i][j].toFixed(precision);
            }
            str += ']';
            if (i < this.rows - 1) str += ',\n';
        }
        str += ']';
        return str;
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = Matrix;
}
