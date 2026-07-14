/**
 * Main Application Entry Point
 * Initializes all calculator components
 */

document.addEventListener('DOMContentLoaded', () => {
    // Initialize core calculator
    window.calculator = new CalculatorCore();
    
    // Initialize UI controller
    window.ui = new UIController(window.calculator);
    
    // Initialize keyboard handler
    window.keyboardHandler = new KeyboardHandler(window.calculator, window.ui);
    
    // Initialize graphing if canvas exists
    const graphCanvas = document.getElementById('graph-canvas');
    if (graphCanvas) {
        window.graphing = new Graphing(graphCanvas);
        
        // Bind graph controls
        document.getElementById('graph-plot')?.addEventListener('click', () => {
            const expr = document.getElementById('graph-function').value;
            if (expr) {
                const colors = ['#00ff88', '#ff6b6b', '#4ecdc4', '#ffe66d', '#95e1d3'];
                const color = colors[window.graphing.functions.length % colors.length];
                window.graphing.addFunction(expr, color);
            }
        });
        
        document.getElementById('graph-clear')?.addEventListener('click', () => {
            window.graphing.clearFunctions();
        });
        
        document.getElementById('graph-zoom-in')?.addEventListener('click', () => {
            window.graphing.zoom(0.8);
            window.graphing.render();
        });
        
        document.getElementById('graph-zoom-out')?.addEventListener('click', () => {
            window.graphing.zoom(1.25);
            window.graphing.render();
        });
    }
    
    // Initialize unit converter
    initUnitConverter();
    
    // Initialize equation solver inputs
    initSolver();
    
    // Initialize matrix editors
    initMatrixMode();
    
    // Setup settings modal
    initSettings();
    
    console.log('Scientific Calculator Pro initialized successfully!');
});

// Unit Converter initialization
function initUnitConverter() {
    const categorySelect = document.getElementById('conv-category');
    const fromSelect = document.getElementById('conv-from');
    const toSelect = document.getElementById('conv-to');
    
    if (!categorySelect || !fromSelect || !toSelect) return;
    
    function updateUnits() {
        const category = categorySelect.value;
        const units = UnitConverter.getUnits(category);
        
        fromSelect.innerHTML = units.map(u => `<option value="${u}">${u}</option>`).join('');
        toSelect.innerHTML = units.map(u => `<option value="${u}">${u}</option>`).join('');
        
        if (units.length > 1) {
            toSelect.selectedIndex = 1;
        }
    }
    
    categorySelect.addEventListener('change', updateUnits);
    updateUnits();
    
    document.getElementById('convert-btn')?.addEventListener('click', () => {
        const value = parseFloat(document.getElementById('conv-value').value);
        const from = fromSelect.value;
        const to = toSelect.value;
        const category = categorySelect.value;
        
        if (isNaN(value)) return;
        
        try {
            const result = UnitConverter.convert(value, from, to, category);
            document.getElementById('converter-result').textContent = 
                `${value} ${from} = ${result.toFixed(6)} ${to}`;
        } catch (error) {
            document.getElementById('converter-result').textContent = 'Error: ' + error.message;
        }
    });
}

// Equation Solver initialization
function initSolver() {
    const solverInputs = document.getElementById('solver-inputs');
    const solverTypes = document.querySelectorAll('.solver-type');
    
    if (!solverInputs || !solverTypes.length) return;
    
    let currentType = 'linear';
    
    function renderInputs(type) {
        switch (type) {
            case 'linear':
                solverInputs.innerHTML = `
                    <input type="number" id="eq-a" placeholder="a" value="1"> x + 
                    <input type="number" id="eq-b" placeholder="b" value="0"> = 0
                `;
                break;
            case 'quadratic':
                solverInputs.innerHTML = `
                    <input type="number" id="eq-a" placeholder="a" value="1"> x² + 
                    <input type="number" id="eq-b" placeholder="b" value="0"> x + 
                    <input type="number" id="eq-c" placeholder="c" value="0"> = 0
                `;
                break;
            case 'cubic':
                solverInputs.innerHTML = `
                    <input type="number" id="eq-a" placeholder="a" value="1"> x³ + 
                    <input type="number" id="eq-b" placeholder="b" value="0"> x² + 
                    <input type="number" id="eq-c" placeholder="c" value="0"> x + 
                    <input type="number" id="eq-d" placeholder="d" value="0"> = 0
                `;
                break;
            case 'system':
                solverInputs.innerHTML = `
                    <div style="margin-bottom: 10px;">
                        <input type="number" id="eq-a1" placeholder="a₁" value="1"> x + 
                        <input type="number" id="eq-b1" placeholder="b₁" value="0"> = 
                        <input type="number" id="eq-c1" placeholder="c₁" value="0">
                    </div>
                    <div>
                        <input type="number" id="eq-a2" placeholder="a₂" value="0"> x + 
                        <input type="number" id="eq-b2" placeholder="b₂" value="1"> = 
                        <input type="number" id="eq-c2" placeholder="c₂" value="0">
                    </div>
                `;
                break;
        }
    }
    
    solverTypes.forEach(btn => {
        btn.addEventListener('click', () => {
            solverTypes.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            currentType = btn.dataset.solver;
            renderInputs(currentType);
        });
    });
    
    renderInputs('linear');
    
    document.getElementById('solve-equation')?.addEventListener('click', () => {
        let result;
        
        switch (currentType) {
            case 'linear':
                result = EquationSolver.linear(
                    parseFloat(document.getElementById('eq-a').value),
                    parseFloat(document.getElementById('eq-b').value)
                );
                break;
            case 'quadratic':
                result = EquationSolver.quadratic(
                    parseFloat(document.getElementById('eq-a').value),
                    parseFloat(document.getElementById('eq-b').value),
                    parseFloat(document.getElementById('eq-c').value)
                );
                break;
            case 'cubic':
                result = EquationSolver.cubic(
                    parseFloat(document.getElementById('eq-a').value),
                    parseFloat(document.getElementById('eq-b').value),
                    parseFloat(document.getElementById('eq-c').value),
                    parseFloat(document.getElementById('eq-d').value)
                );
                break;
            case 'system':
                result = EquationSolver.system2x2(
                    parseFloat(document.getElementById('eq-a1').value),
                    parseFloat(document.getElementById('eq-b1').value),
                    parseFloat(document.getElementById('eq-c1').value),
                    parseFloat(document.getElementById('eq-a2').value),
                    parseFloat(document.getElementById('eq-b2').value),
                    parseFloat(document.getElementById('eq-c2').value)
                );
                break;
        }
        
        displaySolverResult(result);
    });
}

function displaySolverResult(result) {
    const resultEl = document.getElementById('solver-result');
    if (!resultEl) return;
    
    if (!result) {
        resultEl.textContent = 'No solution found';
        return;
    }
    
    let html = '';
    switch (result.type) {
        case 'single':
            html = `x = ${result.solutions[0]}`;
            break;
        case 'two_real':
            html = `x₁ = ${result.solutions[0]}<br>x₂ = ${result.solutions[1]}`;
            break;
        case 'one_real':
            html = `x = ${result.solutions[0]} (double root)`;
            break;
        case 'complex':
            html = `x₁ = ${result.solutions[0].re} + ${result.solutions[0].im}i<br>` +
                   `x₂ = ${result.solutions[1].re} - ${Math.abs(result.solutions[1].im)}i`;
            break;
        case 'three_real':
            html = result.solutions.map((s, i) => `x${i+1} = ${s}`).join('<br>');
            break;
        case 'unique':
            if (result.solutions.x !== undefined) {
                html = `x = ${result.solutions.x}<br>y = ${result.solutions.y}`;
            } else {
                html = result.solutions.map((s, i) => `x${i+1} = ${s}`).join('<br>');
            }
            break;
        case 'infinite':
            html = 'Infinite solutions';
            break;
        case 'none':
            html = 'No solution';
            break;
        default:
            html = JSON.stringify(result);
    }
    
    resultEl.innerHTML = html;
}

// Matrix mode initialization
function initMatrixMode() {
    const createBtnA = document.getElementById('create-matrix-a');
    const createBtnB = document.getElementById('create-matrix-b');
    const editorsContainer = document.getElementById('matrix-editors');
    
    if (!createBtnA || !editorsContainer) return;
    
    window.matrixA = null;
    window.matrixB = null;
    
    function createMatrixEditor(name, rows, cols) {
        let html = `<div class="matrix-editor" data-matrix="${name}">
            <h4>Matrix ${name} (${rows}×${cols})</h4>
            <div class="matrix-grid" style="grid-template-columns: repeat(${cols}, 1fr)">`;
        
        for (let i = 0; i < rows; i++) {
            for (let j = 0; j < cols; j++) {
                html += `<input type="number" data-row="${i}" data-col="${j}" value="0">`;
            }
        }
        
        html += '</div></div>';
        return html;
    }
    
    createBtnA.addEventListener('click', () => {
        const rows = parseInt(document.getElementById('matrix-a-rows').value);
        const cols = parseInt(document.getElementById('matrix-a-cols').value);
        editorsContainer.innerHTML = createMatrixEditor('A', rows, cols);
    });
    
    // Matrix operations
    document.querySelector('.matrix-operations')?.addEventListener('click', (e) => {
        const btn = e.target;
        if (!btn.classList.contains('func-btn')) return;
        
        // Get matrix values from inputs
        const getMatrixFromInputs = (name) => {
            const editor = editorsContainer.querySelector(`[data-matrix="${name}"]`);
            if (!editor) return null;
            
            const inputs = editor.querySelectorAll('input');
            const rows = Math.sqrt(inputs.length); // Assuming square for simplicity
            const size = Math.max(...Array.from(inputs).map(i => parseInt(i.dataset.row))) + 1;
            const cols = Math.max(...Array.from(inputs).map(i => parseInt(i.dataset.col))) + 1;
            
            const data = Array(size).fill(null).map(() => Array(cols).fill(0));
            inputs.forEach(input => {
                data[parseInt(input.dataset.row)][parseInt(input.dataset.col)] = parseFloat(input.value) || 0;
            });
            
            return new Matrix(size, cols, data);
        };
        
        try {
            let result;
            const A = getMatrixFromInputs('A');
            const B = getMatrixFromInputs('B');
            
            switch (btn.dataset.action) {
                case 'matrix-add':
                    result = A.add(B);
                    break;
                case 'matrix-sub':
                    result = A.subtract(B);
                    break;
                case 'matrix-mul':
                    result = A.multiply(B);
                    break;
                case 'matrix-det':
                    result = A.determinant();
                    break;
                case 'matrix-inv':
                    result = A.inverse();
                    break;
                case 'matrix-trans':
                    result = A.transpose();
                    break;
                case 'matrix-rank':
                    result = A.rank();
                    break;
            }
            
            // Display result
            if (typeof result === 'number') {
                document.getElementById('solver-result').textContent = `Result: ${result}`;
            } else {
                document.getElementById('solver-result').textContent = `Result:\n${result.toString()}`;
            }
        } catch (error) {
            document.getElementById('solver-result').textContent = 'Error: ' + error.message;
        }
    });
}

// Settings initialization
function initSettings() {
    const precisionSlider = document.getElementById('precision-slider');
    const precisionValue = document.getElementById('precision-value');
    const fontSizeSlider = document.getElementById('font-size-slider');
    const fontSizeValue = document.getElementById('font-size-value');
    const highContrastCheckbox = document.getElementById('high-contrast');
    const exportBtn = document.getElementById('export-history');
    const closeSettingsBtn = document.getElementById('close-settings');
    
    if (precisionSlider && precisionValue) {
        precisionSlider.addEventListener('input', () => {
            precisionValue.textContent = precisionSlider.value;
            window.calculator.precision = parseInt(precisionSlider.value);
        });
    }
    
    if (fontSizeSlider && fontSizeValue) {
        fontSizeSlider.addEventListener('input', () => {
            fontSizeValue.textContent = fontSizeSlider.value;
            document.documentElement.style.setProperty('--font-size-base', fontSizeSlider.value + 'px');
        });
    }
    
    if (highContrastCheckbox) {
        highContrastCheckbox.addEventListener('change', () => {
            document.documentElement.setAttribute('data-high-contrast', highContrastCheckbox.checked);
        });
    }
    
    if (exportBtn) {
        exportBtn.addEventListener('click', () => {
            window.ui.exportHistory();
        });
    }
    
    if (closeSettingsBtn) {
        closeSettingsBtn.addEventListener('click', () => {
            document.getElementById('settings-modal').classList.remove('active');
        });
    }
}
