/**
 * Calculator Core - Main calculator logic and state management
 */

class CalculatorCore {
    constructor() {
        this.parser = new ExpressionParser();
        this.expression = '';
        this.result = 0;
        this.memory = 0;
        this.memorySlots = {};
        this.history = [];
        this.angleMode = 'DEG'; // DEG, RAD, GRAD
        this.complexMode = false;
        this.fractionMode = false;
        this.scientificNotation = false;
        this.precision = 50;
        this.constants = {
            'pi': Math.PI,
            'π': Math.PI,
            'e': Math.E,
            'φ': 1.618033988749895,
            'phi': 1.618033988749895,
            'γ': 0.5772156649015329,
            'c': 299792458,
            'G': 6.67430e-11,
            'h': 6.62607015e-34,
            'NA': 6.02214076e23,
            'kB': 1.380649e-23,
            'R': 8.314462618
        };
        this.userConstants = {};
        this.userFunctions = {};
        this.lastAnswer = 0;
        
        this.parser.setAngleMode(this.angleMode);
    }

    append(value) {
        this.expression += value;
        return this.expression;
    }

    clear() {
        this.expression = '';
    }

    clearEntry() {
        // Remove last number or operator
        this.expression = this.expression.replace(/[\d.]+|[+\-*/()]|\w+$/g, '');
    }

    backspace() {
        this.expression = this.expression.slice(0, -1);
    }

    calculate() {
        try {
            let expr = this.expression;
            
            // Replace display symbols with JS operators
            expr = expr.replace(/×/g, '*')
                      .replace(/÷/g, '/')
                      .replace(/−/g, '-')
                      .replace(/\^/g, '**');
            
            // Handle implicit multiplication (e.g., 2π, 3(5+2))
            expr = expr.replace(/(\d)([a-zA-Z(πφ])/g, '$1*$2');
            expr = expr.replace(/([)])(\d)/g, '$1*$2');
            expr = expr.replace(/([)])([(])/g, '$1*$2');
            
            // Evaluate
            const result = this.parser.parse(expr);
            this.result = result;
            this.lastAnswer = result;
            
            // Add to history
            this.addToHistory(this.expression, result);
            
            return result;
        } catch (error) {
            throw error;
        }
    }

    addToHistory(expression, result) {
        this.history.unshift({
            expression,
            result,
            timestamp: Date.now()
        });
        // Limit history size
        if (this.history.length > 1000) {
            this.history.pop();
        }
    }

    getHistory() {
        return this.history;
    }

    clearHistory() {
        this.history = [];
    }

    // Memory functions
    memoryClear() {
        this.memory = 0;
    }

    memoryRecall() {
        return this.memory;
    }

    memoryStore(value) {
        this.memory = value !== undefined ? value : this.result;
    }

    memoryAdd(value) {
        this.memory += value !== undefined ? value : this.result;
    }

    memorySubtract(value) {
        this.memory -= value !== undefined ? value : this.result;
    }

    // Angle mode
    setAngleMode(mode) {
        this.angleMode = mode;
        this.parser.setAngleMode(mode);
        return mode;
    }

    toggleAngleMode() {
        const modes = ['DEG', 'RAD', 'GRAD'];
        const currentIndex = modes.indexOf(this.angleMode);
        const nextMode = modes[(currentIndex + 1) % modes.length];
        return this.setAngleMode(nextMode);
    }

    // Constants
    getConstant(name) {
        return this.userConstants[name] || this.constants[name];
    }

    setConstant(name, value) {
        this.userConstants[name] = value;
    }

    // Variables
    storeVariable(name, value) {
        this.parser.setVariable(name, value);
    }

    recallVariable(name) {
        return this.parser.getVariable(name);
    }

    // Scientific functions
    sin(x) { return Math.sin(this.toRadians(x)); }
    cos(x) { return Math.cos(this.toRadians(x)); }
    tan(x) { return Math.tan(this.toRadians(x)); }
    asin(x) { return this.fromRadians(Math.asin(x)); }
    acos(x) { return this.fromRadians(Math.acos(x)); }
    atan(x) { return this.fromRadians(Math.atan(x)); }
    
    sinh(x) { return Math.sinh(x); }
    cosh(x) { return Math.cosh(x); }
    tanh(x) { return Math.tanh(x); }
    asinh(x) { return Math.asinh(x); }
    acosh(x) { return Math.acosh(x); }
    atanh(x) { return Math.atanh(x); }

    ln(x) { return Math.log(x); }
    log10(x) { return Math.log10(x); }
    log2(x) { return Math.log2(x); }
    logBase(x, base) { return Math.log(x) / Math.log(base); }

    sqrt(x) { return Math.sqrt(x); }
    cbrt(x) { return Math.cbrt(x); }
    pow(base, exp) { return Math.pow(base, exp); }

    factorial(n) {
        if (n < 0) throw new Error('Factorial of negative number');
        if (n > 170) return Infinity;
        if (!Number.isInteger(n)) return this.gamma(n + 1);
        if (n <= 1) return 1;
        let result = 1;
        for (let i = 2; i <= n; i++) result *= i;
        return result;
    }

    gamma(z) {
        if (z < 0.5) return Math.PI / (Math.sin(Math.PI * z) * this.gamma(1 - z));
        z -= 1;
        const g = 7;
        const c = [0.99999999999980993, 676.5203681218851, -1259.1392167224028,
                   771.32342877765313, -176.61502916214059, 12.507343278686905,
                   -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7];
        let x = c[0];
        for (let i = 1; i < g + 2; i++) x += c[i] / (z + i);
        return Math.sqrt(2 * Math.PI) * Math.pow(z + g + 0.5, z + 0.5) * Math.exp(-(z + g + 0.5)) * x;
    }

    abs(x) { return Math.abs(x); }
    reciprocal(x) { return 1 / x; }
    percent(x) { return x / 100; }
    negate(x) { return -x; }
    mod(a, b) { return a % b; }

    toRadians(x) {
        if (this.angleMode === 'DEG') return x * Math.PI / 180;
        if (this.angleMode === 'GRAD') return x * Math.PI / 200;
        return x;
    }

    fromRadians(x) {
        if (this.angleMode === 'DEG') return x * 180 / Math.PI;
        if (this.angleMode === 'GRAD') return x * 200 / Math.PI;
        return x;
    }

    // Number system conversions
    toBinary(n) { return (n >>> 0).toString(2); }
    toOctal(n) { return (n >>> 0).toString(8); }
    toHex(n) { return (n >>> 0).toString(16).toUpperCase(); }
    fromBinary(str) { return parseInt(str, 2); }
    fromOctal(str) { return parseInt(str, 8); }
    fromHex(str) { return parseInt(str, 16); }

    // Bitwise operations
    and(a, b) { return a & b; }
    or(a, b) { return a | b; }
    xor(a, b) { return a ^ b; }
    not(a) { return ~a; }
    lshift(a, n) { return a << n; }
    rshift(a, n) { return a >> n; }

    // Random functions
    random() { return Math.random(); }
    randomInt(min, max) { return Math.floor(Math.random() * (max - min + 1)) + min; }
    randomSeed(seed) { 
        // Simple LCG random with seed
        this.seed = seed;
        return () => {
            this.seed = (this.seed * 1103515245 + 12345) & 0x7fffffff;
            return this.seed / 0x7fffffff;
        };
    }

    // Format result
    formatResult(value, options = {}) {
        if (options.scientific || this.scientificNotation) {
            return value.toExponential(options.precision || 10);
        }
        if (options.precision) {
            return parseFloat(value.toFixed(options.precision));
        }
        return value.toString();
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = CalculatorCore;
}
