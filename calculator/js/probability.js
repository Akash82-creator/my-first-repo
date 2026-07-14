/**
 * Probability Calculator - Distributions and combinatorics
 */

class Probability {
    // Factorial
    static factorial(n) {
        if (n < 0) throw new Error('Factorial of negative number');
        if (n > 170) return Infinity;
        if (n <= 1) return 1;
        let result = 1;
        for (let i = 2; i <= n; i++) result *= i;
        return result;
    }

    // Permutations: nPr = n! / (n-r)!
    static permutation(n, r) {
        if (r < 0 || r > n) return 0;
        return Probability.factorial(n) / Probability.factorial(n - r);
    }

    // Combinations: nCr = n! / (r! * (n-r)!)
    static combination(n, r) {
        if (r < 0 || r > n) return 0;
        return Probability.factorial(n) / (Probability.factorial(r) * Probability.factorial(n - r));
    }

    // Binomial distribution: P(X = k) = C(n,k) * p^k * (1-p)^(n-k)
    static binomialPMF(n, p, k) {
        if (k < 0 || k > n) return 0;
        return Probability.combination(n, k) * Math.pow(p, k) * Math.pow(1 - p, n - k);
    }

    // Binomial CDF: P(X ≤ k)
    static binomialCDF(n, p, k) {
        let sum = 0;
        for (let i = 0; i <= k; i++) {
            sum += Probability.binomialPMF(n, p, i);
        }
        return sum;
    }

    // Normal distribution PDF
    static normalPDF(x, mu = 0, sigma = 1) {
        const coeff = 1 / (sigma * Math.sqrt(2 * Math.PI));
        const exponent = -0.5 * Math.pow((x - mu) / sigma, 2);
        return coeff * Math.exp(exponent);
    }

    // Normal distribution CDF (using error function approximation)
    static normalCDF(x, mu = 0, sigma = 1) {
        const z = (x - mu) / (sigma * Math.sqrt(2));
        return 0.5 * (1 + Probability.erf(z));
    }

    // Error function approximation (Abramowitz and Stegun)
    static erf(x) {
        const sign = x >= 0 ? 1 : -1;
        x = Math.abs(x);
        
        const a1 = 0.254829592;
        const a2 = -0.284496736;
        const a3 = 1.421413741;
        const a4 = -1.453152027;
        const a5 = 1.061405429;
        const p = 0.3275911;
        
        const t = 1.0 / (1.0 + p * x);
        const y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * Math.exp(-x * x);
        
        return sign * y;
    }

    // Inverse normal CDF (quantile function)
    static normalInvCDF(p, mu = 0, sigma = 1) {
        if (p <= 0) return -Infinity;
        if (p >= 1) return Infinity;
        
        // Rational approximation for inverse normal
        const a = [
            -3.969683028665376e+01, 2.209460984245205e+02,
            -2.759285104469687e+02, 1.383577518672690e+02,
            -3.066479806614716e+01, 2.506628277459239e+00
        ];
        const b = [
            -5.447609879822406e+01, 1.615858368580409e+02,
            -1.556989798598866e+02, 6.680131188771972e+01,
            -1.328068155288572e+01
        ];
        const c = [
            -7.784894002430293e-03, -3.223964580411365e-01,
            -2.400758277161838e+00, -2.549732539343734e+00,
            4.374664141464968e+00, 2.938163982698783e+00
        ];
        const d = [
            7.784695709041462e-03, 3.224671290700398e-01,
            2.445134137142996e+00, 3.754408661907416e+00
        ];
        
        const pLow = 0.02425;
        const pHigh = 1 - pLow;
        
        let q, r;
        if (p < pLow) {
            q = Math.sqrt(-2 * Math.log(p));
            return (((((c[0]*q+c[1])*q+c[2])*q+c[3])*q+c[4])*q+c[5]) / 
                   ((((d[0]*q+d[1])*q+d[2])*q+d[3])*q+1);
        } else if (p <= pHigh) {
            q = p - 0.5;
            r = q * q;
            return (((((a[0]*r+a[1])*r+a[2])*r+a[3])*r+a[4])*r+a[5])*q / 
                   (((((b[0]*r+b[1])*r+b[2])*r+b[3])*r+b[4])*r+1);
        } else {
            q = Math.sqrt(-2 * Math.log(1 - p));
            return -(((((c[0]*q+c[1])*q+c[2])*q+c[3])*q+c[4])*q+c[5]) / 
                    ((((d[0]*q+d[1])*q+d[2])*q+d[3])*q+1);
        }
    }

    // Poisson distribution: P(X = k) = λ^k * e^(-λ) / k!
    static poissonPMF(lambda, k) {
        if (k < 0) return 0;
        return Math.pow(lambda, k) * Math.exp(-lambda) / Probability.factorial(k);
    }

    // Poisson CDF: P(X ≤ k)
    static poissonCDF(lambda, k) {
        let sum = 0;
        for (let i = 0; i <= k; i++) {
            sum += Probability.poissonPMF(lambda, i);
        }
        return sum;
    }

    // Geometric distribution: P(X = k) = (1-p)^(k-1) * p
    static geometricPMF(p, k) {
        if (k < 1) return 0;
        return Math.pow(1 - p, k - 1) * p;
    }

    // Geometric CDF: P(X ≤ k)
    static geometricCDF(p, k) {
        if (k < 1) return 0;
        return 1 - Math.pow(1 - p, k);
    }

    // Hypergeometric distribution
    // N = population size, K = success states, n = draws, k = observed successes
    static hypergeometricPMF(N, K, n, k) {
        if (k < 0 || k > K || k > n) return 0;
        if (n - k > N - K) return 0;
        
        return Probability.combination(K, k) * Probability.combination(N - K, n - k) / 
               Probability.combination(N, n);
    }

    // Hypergeometric CDF
    static hypergeometricCDF(N, K, n, k) {
        let sum = 0;
        for (let i = 0; i <= k; i++) {
            sum += Probability.hypergeometricPMF(N, K, n, i);
        }
        return sum;
    }

    // Chi-squared distribution (approximation using gamma)
    static chiSquaredPDF(x, k) {
        if (x < 0) return 0;
        return Math.pow(x, k/2 - 1) * Math.exp(-x/2) / 
               (Math.pow(2, k/2) * Probability.gamma(k/2));
    }

    // Gamma function (Lanczos approximation)
    static gamma(z) {
        if (z < 0.5) {
            return Math.PI / (Math.sin(Math.PI * z) * Probability.gamma(1 - z));
        }
        z -= 1;
        const g = 7;
        const c = [
            0.99999999999980993, 676.5203681218851, -1259.1392167224028,
            771.32342877765313, -176.61502916214059, 12.507343278686905,
            -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7
        ];
        let x = c[0];
        for (let i = 1; i < g + 2; i++) {
            x += c[i] / (z + i);
        }
        return Math.sqrt(2 * Math.PI) * Math.pow(z + g + 0.5, z + 0.5) * Math.exp(-(z + g + 0.5)) * x;
    }

    // Student's t-distribution PDF
    static tPDF(x, nu) {
        const coeff = Probability.gamma((nu + 1) / 2) / 
                      (Math.sqrt(nu * Math.PI) * Probability.gamma(nu / 2));
        return coeff * Math.pow(1 + x * x / nu, -(nu + 1) / 2);
    }

    // Expected value of discrete random variable
    static expectedValue(values, probabilities) {
        if (values.length !== probabilities.length) {
            throw new Error('Arrays must have same length');
        }
        return values.reduce((sum, v, i) => sum + v * probabilities[i], 0);
    }

    // Variance of discrete random variable
    static variance(values, probabilities) {
        const mean = Probability.expectedValue(values, probabilities);
        return values.reduce((sum, v, i) => sum + Math.pow(v - mean, 2) * probabilities[i], 0);
    }

    // Standard deviation
    static stdDev(values, probabilities) {
        return Math.sqrt(Probability.variance(values, probabilities));
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = Probability;
}
