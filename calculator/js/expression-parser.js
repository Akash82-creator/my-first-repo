/**
 * Expression Parser and Evaluator
 * Handles mathematical expressions with proper operator precedence
 */

class ExpressionParser {
    constructor() {
        this.operators = {
            '+': { precedence: 1, associativity: 'left', arity: 2 },
            '-': { precedence: 1, associativity: 'left', arity: 2 },
            'u-': { precedence: 3, associativity: 'right', arity: 1 }, // unary minus
            '×': { precedence: 2, associativity: 'left', arity: 2 },
            '÷': { precedence: 2, associativity: 'left', arity: 2 },
            '*': { precedence: 2, associativity: 'left', arity: 2 },
            '/': { precedence: 2, associativity: 'left', arity: 2 },
            '%': { precedence: 2, associativity: 'left', arity: 2 },
            '^': { precedence: 4, associativity: 'right', arity: 2 },
            '!': { precedence: 5, associativity: 'right', arity: 1 },
            'mod': { precedence: 2, associativity: 'left', arity: 2 }
        };
        
        this.functions = new Set([
            'sin', 'cos', 'tan', 'asin', 'acos', 'atan',
            'sinh', 'cosh', 'tanh', 'asinh', 'acosh', 'atanh',
            'ln', 'log', 'log10', 'log2', 'sqrt', 'cbrt',
            'abs', 'exp', 'floor', 'ceil', 'round', 'sign',
            'factorial', 'gamma', 'erf', 'erfc'
        ]);
        
        this.constants = {
            'π': Math.PI,
            'pi': Math.PI,
            'e': Math.E,
            'φ': 1.618033988749895,
            'phi': 1.618033988749895,
            'γ': 0.5772156649015329,
            'gamma': 0.5772156649015329,
            'ans': 0
        };
        
        this.angleMode = 'DEG'; // DEG, RAD, GRAD
        this.variables = {};
        this.lastAnswer = 0;
    }

    setAngleMode(mode) {
        this.angleMode = mode;
    }

    setVariable(name, value) {
        this.variables[name] = value;
    }

    getVariable(name) {
        return this.variables[name];
    }

    tokenize(expression) {
        const tokens = [];
        let i = 0;
        expression = expression.replace(/\s+/g, '');
        
        while (i < expression.length) {
            const char = expression[i];
            
            // Numbers (including scientific notation)
            if (/\d/.test(char) || (char === '.' && /\d/.test(expression[i + 1]))) {
                let num = '';
                while (i < expression.length && (/\d/.test(expression[i]) || expression[i] === '.')) {
                    num += expression[i++];
                }
                // Handle scientific notation
                if (i < expression.length && (expression[i] === 'e' || expression[i] === 'E')) {
                    let exp = expression[i++];
                    if (i < expression.length && (expression[i] === '+' || expression[i] === '-')) {
                        exp += expression[i++];
                    }
                    while (i < expression.length && /\d/.test(expression[i])) {
                        exp += expression[i++];
                    }
                    num += exp;
                }
                tokens.push({ type: 'number', value: parseFloat(num) });
            }
            // Identifiers (functions, constants, variables)
            else if (/[a-zA-Z_]/.test(char) || char === 'π' || char === 'φ' || char === 'γ') {
                let ident = char;
                i++;
                while (i < expression.length && /[a-zA-Z0-9_]/.test(expression[i])) {
                    ident += expression[i++];
                }
                
                if (this.functions.has(ident.toLowerCase())) {
                    tokens.push({ type: 'function', value: ident.toLowerCase() });
                } else if (ident in this.constants) {
                    tokens.push({ type: 'number', value: this.constants[ident] });
                } else if (ident in this.variables) {
                    tokens.push({ type: 'number', value: this.variables[ident] });
                } else {
                    tokens.push({ type: 'variable', value: ident });
                }
            }
            // Operators
            else if (char in this.operators || char === '+' || char === '-') {
                tokens.push({ type: 'operator', value: char });
                i++;
            }
            // Parentheses
            else if (char === '(') {
                tokens.push({ type: 'lparen' });
                i++;
            }
            else if (char === ')') {
                tokens.push({ type: 'rparen' });
                i++;
            }
            // Comma (for function arguments)
            else if (char === ',') {
                tokens.push({ type: 'comma' });
                i++;
            }
            else {
                throw new Error(`Unexpected character: ${char}`);
            }
        }
        
        return tokens;
    }

    toRPN(tokens) {
        const output = [];
        const stack = [];
        let prevToken = null;
        
        for (const token of tokens) {
            if (token.type === 'number') {
                output.push(token);
            }
            else if (token.type === 'variable') {
                if (this.variables[token.value] !== undefined) {
                    output.push({ type: 'number', value: this.variables[token.value] });
                } else {
                    throw new Error(`Undefined variable: ${token.value}`);
                }
            }
            else if (token.type === 'function') {
                stack.push(token);
            }
            else if (token.type === 'operator') {
                // Handle unary minus
                let op = token.value;
                if (op === '-' && (prevToken === null || 
                    (prevToken.type === 'operator' && prevToken.value !== '!') ||
                    prevToken.type === 'lparen' ||
                    prevToken.type === 'comma')) {
                    op = 'u-';
                }
                
                while (stack.length > 0) {
                    const top = stack[stack.length - 1];
                    if (top.type === 'operator') {
                        const topOp = top.value;
                        const currOpInfo = this.operators[op] || { precedence: 0 };
                        const topOpInfo = this.operators[topOp] || { precedence: 0 };
                        
                        if ((currOpInfo.associativity === 'left' && currOpInfo.precedence <= topOpInfo.precedence) ||
                            (currOpInfo.associativity === 'right' && currOpInfo.precedence < topOpInfo.precedence)) {
                            output.push(stack.pop());
                        } else {
                            break;
                        }
                    } else if (top.type === 'function') {
                        output.push(stack.pop());
                    } else {
                        break;
                    }
                }
                stack.push({ type: 'operator', value: op });
            }
            else if (token.type === 'lparen') {
                stack.push(token);
            }
            else if (token.type === 'rparen') {
                while (stack.length > 0 && stack[stack.length - 1].type !== 'lparen') {
                    output.push(stack.pop());
                }
                if (stack.length === 0) {
                    throw new Error('Mismatched parentheses');
                }
                stack.pop(); // Remove lparen
                
                // If there's a function on top of stack, pop it
                if (stack.length > 0 && stack[stack.length - 1].type === 'function') {
                    output.push(stack.pop());
                }
            }
            else if (token.type === 'comma') {
                while (stack.length > 0 && stack[stack.length - 1].type !== 'lparen') {
                    output.push(stack.pop());
                }
            }
            
            prevToken = token;
        }
        
        while (stack.length > 0) {
            const top = stack.pop();
            if (top.type === 'lparen' || top.type === 'rparen') {
                throw new Error('Mismatched parentheses');
            }
            output.push(top);
        }
        
        return output;
    }

    evaluateRPN(rpn) {
        const stack = [];
        
        for (const token of rpn) {
            if (token.type === 'number') {
                stack.push(token.value);
            }
            else if (token.type === 'operator') {
                if (token.value === 'u-') {
                    const a = stack.pop();
                    stack.push(-a);
                } else if (token.value === '!') {
                    const a = stack.pop();
                    stack.push(this.factorial(a));
                } else {
                    const b = stack.pop();
                    const a = stack.pop();
                    
                    switch (token.value) {
                        case '+': stack.push(a + b); break;
                        case '-': stack.push(a - b); break;
                        case '×': case '*': stack.push(a * b); break;
                        case '÷': case '/': stack.push(a / b); break;
                        case '%': stack.push(a % b); break;
                        case '^': stack.push(Math.pow(a, b)); break;
                        case 'mod': stack.push(a % b); break;
                        default: throw new Error(`Unknown operator: ${token.value}`);
                    }
                }
            }
            else if (token.type === 'function') {
                const args = [];
                while (stack.length > 0 && typeof stack[stack.length - 1] === 'number') {
                    args.unshift(stack.pop());
                }
                
                const result = this.callFunction(token.value, args);
                stack.push(result);
            }
        }
        
        if (stack.length !== 1) {
            throw new Error('Invalid expression');
        }
        
        return stack[0];
    }

    callFunction(name, args) {
        const x = args[0];
        const y = args[1];
        
        // Convert angle if needed
        const toRad = (val) => {
            if (this.angleMode === 'DEG') return val * Math.PI / 180;
            if (this.angleMode === 'GRAD') return val * Math.PI / 200;
            return val;
        };
        
        const fromRad = (val) => {
            if (this.angleMode === 'DEG') return val * 180 / Math.PI;
            if (this.angleMode === 'GRAD') return val * 200 / Math.PI;
            return val;
        };
        
        switch (name) {
            // Trigonometric
            case 'sin': return Math.sin(toRad(x));
            case 'cos': return Math.cos(toRad(x));
            case 'tan': return Math.tan(toRad(x));
            case 'asin': return fromRad(Math.asin(x));
            case 'acos': return fromRad(Math.acos(x));
            case 'atan': return fromRad(Math.atan(x));
            
            // Hyperbolic
            case 'sinh': return Math.sinh(x);
            case 'cosh': return Math.cosh(x);
            case 'tanh': return Math.tanh(x);
            case 'asinh': return Math.asinh(x);
            case 'acosh': return Math.acosh(x);
            case 'atanh': return Math.atanh(x);
            
            // Logarithmic
            case 'ln': return Math.log(x);
            case 'log': return args.length === 2 ? Math.log(x) / Math.log(y) : Math.log10(x);
            case 'log10': return Math.log10(x);
            case 'log2': return Math.log2(x);
            
            // Roots and powers
            case 'sqrt': return Math.sqrt(x);
            case 'cbrt': return Math.cbrt(x);
            case 'exp': return Math.exp(x);
            
            // Other
            case 'abs': return Math.abs(x);
            case 'floor': return Math.floor(x);
            case 'ceil': return Math.ceil(x);
            case 'round': return Math.round(x);
            case 'sign': return Math.sign(x);
            case 'factorial': return this.factorial(x);
            
            default:
                throw new Error(`Unknown function: ${name}`);
        }
    }

    factorial(n) {
        if (n < 0) throw new Error('Factorial of negative number');
        if (n > 170) return Infinity;
        if (!Number.isInteger(n)) {
            // Use gamma function for non-integers
            return this.gamma(n + 1);
        }
        if (n === 0 || n === 1) return 1;
        
        let result = 1;
        for (let i = 2; i <= n; i++) {
            result *= i;
        }
        return result;
    }

    gamma(z) {
        // Lanczos approximation for gamma function
        if (z < 0.5) {
            return Math.PI / (Math.sin(Math.PI * z) * this.gamma(1 - z));
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
        
        const t = z + g + 0.5;
        return Math.sqrt(2 * Math.PI) * Math.pow(t, z + 0.5) * Math.exp(-t) * x;
    }

    parse(expression) {
        try {
            const tokens = this.tokenize(expression);
            const rpn = this.toRPN(tokens);
            const result = this.evaluateRPN(rpn);
            this.lastAnswer = result;
            this.constants['ans'] = result;
            return result;
        } catch (error) {
            throw new Error(`Parse error: ${error.message}`);
        }
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = ExpressionParser;
}
