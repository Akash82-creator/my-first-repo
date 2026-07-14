# Scientific Calculator Pro

A modern, feature-rich web-based scientific calculator that rivals or exceeds the capabilities of Casio fx-991EX, TI-36X Pro, and HP 35s combined.

![Calculator Demo](demo.png)

## Features

### 🎨 Modern Interface
- **Dark/Light Theme** - Toggle between themes with a single click
- **Responsive Design** - Works seamlessly on desktop and mobile devices
- **Realistic Layout** - Professional calculator appearance
- **Multi-line Display** - Shows current expression, result preview, and history
- **Collapsible History Panel** - Access your calculation history easily
- **Mode Indicators** - Visual feedback for DEG/RAD/GRAD, Complex mode, Memory status

### 🔢 Basic & Scientific Operations
- All standard arithmetic operations (+, -, ×, ÷)
- Parentheses with proper nesting
- Percentage, reciprocal, square, cube, arbitrary powers
- Nth roots, modulus, absolute value, negation
- Factorial function
- Full trigonometric functions (sin, cos, tan and inverses)
- Hyperbolic functions (sinh, cosh, tanh and inverses)
- Logarithms (ln, log₁₀, log₂, arbitrary base)
- Support for degrees, radians, and gradians

### 🧮 Advanced Mathematics

#### Complex Numbers
- Rectangular (a+bi) and polar form support
- Conjugate, magnitude, phase operations
- Complex trigonometric functions
- Powers and roots of complex numbers

#### Matrix Calculator (up to 10×10)
- Addition, subtraction, multiplication
- Determinant, inverse, transpose
- Rank calculation
- LU decomposition
- Eigenvalues and eigenvectors
- Gaussian elimination

#### Vector Calculator (2D & 3D)
- Dot product and cross product
- Magnitude and unit vectors
- Angle between vectors
- Vector projection

#### Statistics
- Single-variable: mean, median, mode, variance, standard deviation, quartiles
- Two-variable: correlation, covariance, linear regression, polynomial regression

#### Probability
- Permutations (nPr) and combinations (nCr)
- Binomial distribution
- Normal distribution (PDF and CDF)
- Poisson distribution
- Geometric distribution
- Hypergeometric distribution

#### Calculus
- Numerical differentiation
- Numerical integration (Trapezoidal, Simpson's, Romberg)
- Tangent line calculation
- Root finding (Newton-Raphson, Secant, Bisection methods)

#### Equation Solver
- Linear equations
- Quadratic equations (with complex solutions)
- Cubic equations
- Quartic equations
- Systems of linear equations (2×2, 3×3)
- Nonlinear equations

### 📊 Graphing
- 2D function plotting
- Multiple functions with different colors
- Zoom in/out functionality
- Pan support
- Find intersections
- Grid with axis labels

### 🔄 Unit Converter
Convert between units in these categories:
- Length (including light years, astronomical units)
- Area
- Volume
- Mass
- Temperature
- Pressure
- Energy
- Power
- Time
- Speed
- Data storage

### 💰 Financial Calculator
- Compound interest
- Continuous compounding
- Loan EMI calculation
- Amortization schedules
- Future/Present value
- CAGR (Compound Annual Growth Rate)
- ROI (Return on Investment)
- Depreciation calculations
- NPV and IRR analysis

### 👨‍💻 Programmer Mode
- Binary, Octal, Decimal, Hexadecimal conversions
- Bitwise operations (AND, OR, XOR, NOT)
- Left/Right shift operations
- IEEE-754 floating-point decoder

### ⌨️ Keyboard Shortcuts

| Key | Function |
|-----|----------|
| 0-9 | Numbers |
| + - * / | Basic operators |
| Enter / = | Calculate |
| Escape | Clear all |
| Backspace | Delete last character |
| ( ) | Parentheses |
| F1 | Toggle angle mode |
| F2 | Natural log (ln) |
| F3 | Base-10 log |
| F4-F6 | sin, cos, tan |
| F7 | Square root |
| F8 | Factorial |
| F9 | Square |
| F10 | Power |
| s, c, t | sin, cos, tan shortcuts |
| p | Pi constant |
| Ctrl+H | Toggle history |
| Ctrl+T | Toggle theme |
| Ctrl+E | Export history |

### 💾 Memory Functions
- MC (Memory Clear)
- MR (Memory Recall)
- MS (Memory Store)
- M+ (Memory Add)
- M- (Memory Subtract)
- Multiple memory slots support

### 📜 History
- Unlimited calculation history
- Click any history item to reuse
- Search functionality
- Export as JSON

### ⚡ Performance
- Response time < 10ms for normal calculations
- Optimized rendering with no unnecessary re-renders
- Efficient expression parser using Shunting Yard algorithm
- Lazy loading where appropriate

### ♿ Accessibility
- Full keyboard navigation
- Screen reader support with ARIA labels
- High contrast mode
- Adjustable font size (12-24px)
- Focus indicators

## Installation

Simply open `index.html` in any modern web browser. No installation or server required!

```bash
# Clone the repository
git clone <repository-url>
cd calculator

# Open in browser
open index.html
```

Or use a local server:

```bash
# Using Python
python -m http.server 8000

# Using Node.js
npx serve
```

## File Structure

```
calculator/
├── index.html          # Main HTML file
├── css/
│   └── styles.css      # All styles with CSS variables for theming
├── js/
│   ├── bigint-decimal.js    # Arbitrary precision arithmetic
│   ├── expression-parser.js # Expression parsing and evaluation
│   ├── calculator-core.js   # Core calculator logic
│   ├── matrix.js            # Matrix operations
│   ├── vector.js            # Vector operations
│   ├── statistics.js        # Statistical functions
│   ├── probability.js       # Probability distributions
│   ├── calculus.js          # Calculus operations
│   ├── solver.js            # Equation solvers
│   ├── converter.js         # Unit converter
│   ├── financial.js         # Financial calculations
│   ├── graphing.js          # 2D graphing
│   ├── ui-controller.js     # UI management
│   ├── keyboard-handler.js  # Keyboard shortcuts
│   └── main.js              # Application entry point
└── README.md           # This file
```

## Usage Examples

### Basic Calculation
```
2 + 2 × 3 = 8
(5 + 3)² = 64
```

### Scientific Functions
```
sin(30) = 0.5 (in DEG mode)
√144 = 12
log₁₀(100) = 2
5! = 120
```

### Implicit Multiplication
```
2π ≈ 6.283
3(5+2) = 21
sin(45)² + cos(45)² = 1
```

### Matrix Operations
1. Switch to Matrix mode
2. Create matrices A and B
3. Select operation (A+B, A×B, det(A), etc.)

### Graphing
1. Expand the graphing panel
2. Enter function: `sin(x)` or `x^2 - 4`
3. Click Plot
4. Use zoom buttons to adjust view

## Browser Support

- Chrome 80+
- Firefox 75+
- Safari 13+
- Edge 80+

## Technical Details

### Precision
- Standard calculations use JavaScript's native double precision
- Arbitrary precision mode supports up to 100 decimal digits
- BigDecimal implementation for high-precision arithmetic

### Constants Available
- π (Pi)
- e (Euler's number)
- φ (Golden ratio)
- γ (Euler-Mascheroni constant)
- c (Speed of light)
- G (Gravitational constant)
- h (Planck constant)
- Nₐ (Avogadro's number)
- kᵦ (Boltzmann constant)
- R (Gas constant)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - Feel free to use this project for personal or commercial purposes.

## Acknowledgments

Inspired by:
- Casio fx-991EX ClassWiz
- Texas Instruments TI-36X Pro
- HP 35s Scientific Calculator

---

Built with ❤️ using pure HTML, CSS, and JavaScript - No external dependencies!
