/**
 * BigDecimal implementation for arbitrary precision arithmetic
 * Supports up to 100 decimal digits of precision
 */

class BigDecimal {
    constructor(value, precision = 50) {
        this.precision = precision;
        if (value instanceof BigDecimal) {
            this.value = value.value;
            this.scale = value.scale;
            this.sign = value.sign;
        } else if (typeof value === 'string') {
            this.parseString(value);
        } else if (typeof value === 'number') {
            this.parseNumber(value);
        } else {
            this.value = 0n;
            this.scale = 0;
            this.sign = 1;
        }
        this.normalize();
    }

    parseString(str) {
        str = str.trim();
        this.sign = str.startsWith('-') ? -1 : 1;
        if (str.startsWith('-') || str.startsWith('+')) {
            str = str.slice(1);
        }
        
        const parts = str.split('.');
        const intPart = parts[0] || '0';
        const fracPart = parts[1] || '';
        
        this.scale = fracPart.length;
        this.value = BigInt(intPart + fracPart);
    }

    parseNumber(num) {
        if (!isFinite(num)) {
            this.value = 0n;
            this.scale = 0;
            this.sign = num > 0 ? 1 : -1;
            return;
        }
        
        this.sign = num < 0 ? -1 : 1;
        num = Math.abs(num);
        
        const str = num.toExponential(15);
        const [mantissa, exp] = str.split('e');
        const [intPart, fracPart] = mantissa.split('.');
        
        const exponent = parseInt(exp);
        const fullStr = intPart + (fracPart || '');
        
        const actualScale = Math.max(0, exponent + (fracPart ? fracPart.length : 0));
        this.scale = Math.min(actualScale, this.precision);
        
        let intStr = fullStr;
        if (exponent >= 0) {
            intStr = fullStr + '0'.repeat(exponent - (fracPart ? fracPart.length : 0));
        }
        
        this.value = BigInt(intStr || '0');
    }

    normalize() {
        while (this.value !== 0n && this.scale > 0 && this.value % 10n === 0n) {
            this.value /= 10n;
            this.scale--;
        }
        if (this.value === 0n) {
            this.scale = 0;
            this.sign = 1;
        }
    }

    toString() {
        if (this.value === 0n) return '0';
        
        let str = this.value.toString();
        if (this.scale === 0) {
            return this.sign === -1 ? '-' + str : str;
        }
        
        while (str.length <= this.scale) {
            str = '0' + str;
        }
        
        const intPart = str.slice(0, -this.scale);
        const fracPart = str.slice(-this.scale);
        
        const result = (intPart || '0') + '.' + fracPart.replace(/0+$/, '');
        return this.sign === -1 ? '-' + result : result;
    }

    toNumber() {
        return parseFloat(this.toString());
    }

    add(other) {
        other = other instanceof BigDecimal ? other : new BigDecimal(other, this.precision);
        const maxScale = Math.max(this.scale, other.scale);
        
        const a = this.value * (10n ** BigInt(maxScale - this.scale));
        const b = other.value * (10n ** BigInt(maxScale - other.scale));
        
        const result = new BigDecimal(0, this.precision);
        result.sign = (a * BigInt(this.sign) + b * BigInt(other.sign)) >= 0n ? 1 : -1;
        result.value = BigInt(Math.abs(Number(a * BigInt(this.sign) + b * BigInt(other.sign))));
        result.scale = maxScale;
        result.normalize();
        return result;
    }

    subtract(other) {
        other = other instanceof BigDecimal ? other : new BigDecimal(other, this.precision);
        const negated = new BigDecimal(other);
        negated.sign *= -1;
        return this.add(negated);
    }

    multiply(other) {
        other = other instanceof BigDecimal ? other : new BigDecimal(other, this.precision);
        const result = new BigDecimal(0, this.precision);
        result.sign = this.sign * other.sign;
        result.value = this.value * other.value;
        result.scale = this.scale + other.scale;
        result.normalize();
        return result;
    }

    divide(other) {
        other = other instanceof BigDecimal ? other : new BigDecimal(other, this.precision);
        if (other.value === 0n) {
            throw new Error('Division by zero');
        }
        
        const result = new BigDecimal(0, this.precision);
        result.sign = this.sign * other.sign;
        
        const multiplier = 10n ** BigInt(this.precision);
        const dividend = this.value * multiplier * (10n ** BigInt(other.scale));
        const divisor = other.value * (10n ** BigInt(this.scale));
        
        result.value = dividend / divisor;
        result.scale = this.precision;
        result.normalize();
        return result;
    }

    mod(other) {
        other = other instanceof BigDecimal ? other : new BigDecimal(other, this.precision);
        const quotient = this.divide(other);
        const floorQuotient = Math.floor(quotient.toNumber());
        return this.subtract(other.multiply(new BigDecimal(floorQuotient)));
    }

    pow(exponent) {
        if (exponent === 0) return new BigDecimal(1, this.precision);
        if (exponent < 0) {
            return new BigDecimal(1, this.precision).divide(this.pow(-exponent));
        }
        
        let result = new BigDecimal(1, this.precision);
        let base = new BigDecimal(this);
        let exp = exponent;
        
        while (exp > 0) {
            if (exp % 2 === 1) {
                result = result.multiply(base);
            }
            base = base.multiply(base);
            exp = Math.floor(exp / 2);
        }
        return result;
    }

    sqrt() {
        if (this.sign === -1) {
            throw new Error('Cannot compute square root of negative number');
        }
        if (this.value === 0n) return new BigDecimal(0, this.precision);
        
        // Newton's method for square root
        let x = new BigDecimal(Math.sqrt(this.toNumber()), this.precision);
        for (let i = 0; i < 20; i++) {
            const prev = x;
            x = x.add(this.divide(x)).multiply(new BigDecimal('0.5'));
            if (Math.abs(x.subtract(prev).toNumber()) < 1e-50) break;
        }
        return x;
    }

    cbrt() {
        if (this.value === 0n) return new BigDecimal(0, this.precision);
        
        const sign = this.sign;
        const absValue = new BigDecimal(this);
        absValue.sign = 1;
        
        let x = new BigDecimal(Math.cbrt(absValue.toNumber()), this.precision);
        for (let i = 0; i < 20; i++) {
            const prev = x;
            const x2 = x.multiply(x);
            x = x.multiply(new BigDecimal(2)).add(absValue.divide(x2));
            x = x.multiply(new BigDecimal('0.33333333333333333333333333333333333333333333333333'));
            if (Math.abs(x.subtract(prev).toNumber()) < 1e-50) break;
        }
        
        x.sign = sign;
        return x;
    }

    abs() {
        const result = new BigDecimal(this);
        result.sign = 1;
        return result;
    }

    negate() {
        const result = new BigDecimal(this);
        result.sign *= -1;
        return result;
    }

    factorial() {
        const n = Math.floor(this.toNumber());
        if (n < 0) throw new Error('Factorial of negative number');
        if (n > 170) throw new Error('Factorial overflow');
        
        let result = 1n;
        for (let i = 2; i <= n; i++) {
            result *= BigInt(i);
        }
        
        const bdResult = new BigDecimal(result.toString(), this.precision);
        return bdResult;
    }

    compare(other) {
        other = other instanceof BigDecimal ? other : new BigDecimal(other, this.precision);
        const maxScale = Math.max(this.scale, other.scale);
        
        const a = this.value * BigInt(this.sign) * (10n ** BigInt(maxScale - this.scale));
        const b = other.value * BigInt(other.sign) * (10n ** BigInt(maxScale - other.scale));
        
        if (a > b) return 1;
        if (a < b) return -1;
        return 0;
    }

    equals(other) {
        return this.compare(other) === 0;
    }

    lessThan(other) {
        return this.compare(other) < 0;
    }

    greaterThan(other) {
        return this.compare(other) > 0;
    }

    clone() {
        return new BigDecimal(this);
    }
}

// High-precision mathematical constants
BigDecimal.PI = new BigDecimal('3.14159265358979323846264338327950288419716939937510', 50);
BigDecimal.E = new BigDecimal('2.71828182845904523536028747135266249775724709369995', 50);
BigDecimal.PHI = new BigDecimal('1.61803398874989484820458683436563811772030917980576', 50);
BigDecimal.GAMMA = new BigDecimal('0.57721566490153286060651209008240243104215933593992', 50);

// Physical constants
BigDecimal.C = new BigDecimal('299792458', 50); // Speed of light (m/s)
BigDecimal.G = new BigDecimal('6.67430e-11', 50); // Gravitational constant
BigDecimal.H = new BigDecimal('6.62607015e-34', 50); // Planck constant
BigDecimal.NA = new BigDecimal('6.02214076e23', 50); // Avogadro number
BigDecimal.KB = new BigDecimal('1.380649e-23', 50); // Boltzmann constant
BigDecimal.R = new BigDecimal('8.314462618', 50); // Gas constant

if (typeof module !== 'undefined' && module.exports) {
    module.exports = BigDecimal;
}
