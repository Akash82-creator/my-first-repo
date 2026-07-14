/**
 * UI Controller - Handles all UI interactions and updates
 */

class UIController {
    constructor(calculator) {
        this.calculator = calculator;
        this.theme = 'dark';
        this.highContrast = false;
        
        this.initElements();
        this.bindEvents();
        this.updateDisplay();
    }

    initElements() {
        // Display elements
        this.expressionEl = document.getElementById('expression');
        this.resultEl = document.getElementById('result');
        this.historyPanel = document.getElementById('history-panel');
        this.historyList = document.getElementById('history-list');
        
        // Indicators
        this.angleModeEl = document.getElementById('angle-mode');
        this.complexModeEl = document.getElementById('complex-mode');
        this.sciNotationEl = document.getElementById('sci-notation');
        this.memoryIndicatorEl = document.getElementById('memory-indicator');
        
        // Buttons
        this.themeToggle = document.getElementById('theme-toggle');
        this.collapseHistoryBtn = document.getElementById('collapse-history');
        this.clearHistoryBtn = document.getElementById('clear-history');
        
        // Mode tabs
        this.modeTabs = document.querySelectorAll('.mode-tab');
        this.modeContents = document.querySelectorAll('.mode-content');
        
        // Graphing
        this.graphCanvas = document.getElementById('graph-canvas');
        this.graphPanel = document.getElementById('graphing-panel');
        this.toggleGraphBtn = document.getElementById('toggle-graph');
        
        // Settings modal
        this.settingsModal = document.getElementById('settings-modal');
    }

    bindEvents() {
        // Calculator buttons
        document.querySelectorAll('.btn').forEach(btn => {
            btn.addEventListener('click', (e) => this.handleButtonClick(e));
        });

        // Theme toggle
        this.themeToggle.addEventListener('click', () => this.toggleTheme());

        // History controls
        this.collapseHistoryBtn.addEventListener('click', () => {
            this.historyPanel.classList.toggle('collapsed');
            this.collapseHistoryBtn.textContent = this.historyPanel.classList.contains('collapsed') ? '▼' : '▲';
        });

        this.clearHistoryBtn.addEventListener('click', () => {
            this.calculator.clearHistory();
            this.renderHistory();
        });

        // Mode tabs
        this.modeTabs.forEach(tab => {
            tab.addEventListener('click', () => this.switchMode(tab.dataset.mode));
        });

        // Graphing
        this.toggleGraphBtn.addEventListener('click', () => {
            this.graphPanel.classList.toggle('collapsed');
            this.toggleGraphBtn.textContent = this.graphPanel.classList.contains('collapsed') ? '▲' : '▼';
        });

        // Keyboard support
        document.addEventListener('keydown', (e) => this.handleKeyboard(e));
    }

    handleButtonClick(e) {
        const btn = e.target;
        const value = btn.dataset.value;
        const action = btn.dataset.action;

        if (value !== undefined) {
            this.calculator.append(value);
            this.updateDisplay();
        } else if (action) {
            this.handleAction(action);
        }
    }

    handleAction(action) {
        switch (action) {
            case 'equals':
                this.calculate();
                break;
            case 'clear-all':
                this.calculator.clear();
                this.updateDisplay();
                break;
            case 'clear-entry':
                this.calculator.clearEntry();
                this.updateDisplay();
                break;
            case 'backspace':
                this.calculator.backspace();
                this.updateDisplay();
                break;
            case 'deg-rad-grad':
                const newMode = this.calculator.toggleAngleMode();
                this.angleModeEl.textContent = newMode;
                break;
            case 'mc':
                this.calculator.memoryClear();
                this.updateMemoryIndicator();
                break;
            case 'mr':
                const memVal = this.calculator.memoryRecall();
                this.calculator.append(memVal.toString());
                this.updateDisplay();
                break;
            case 'ms':
                this.calculator.memoryStore();
                this.updateMemoryIndicator();
                break;
            case 'm-plus':
                this.calculator.memoryAdd();
                this.updateMemoryIndicator();
                break;
            case 'm-minus':
                this.calculator.memorySubtract();
                this.updateMemoryIndicator();
                break;
            case 'sin':
                this.calculator.append('sin(');
                this.updateDisplay();
                break;
            case 'cos':
                this.calculator.append('cos(');
                this.updateDisplay();
                break;
            case 'tan':
                this.calculator.append('tan(');
                this.updateDisplay();
                break;
            case 'ln':
                this.calculator.append('ln(');
                this.updateDisplay();
                break;
            case 'log10':
                this.calculator.append('log10(');
                this.updateDisplay();
                break;
            case 'sqrt':
                this.calculator.append('sqrt(');
                this.updateDisplay();
                break;
            case 'power':
                this.calculator.append('^');
                this.updateDisplay();
                break;
            case 'factorial':
                this.calculator.append('!');
                this.updateDisplay();
                break;
            case 'pi':
                this.calculator.append('π');
                this.updateDisplay();
                break;
            case 'e':
                this.calculator.append('e');
                this.updateDisplay();
                break;
            case 'paren-open':
                this.calculator.append('(');
                this.updateDisplay();
                break;
            case 'paren-close':
                this.calculator.append(')');
                this.updateDisplay();
                break;
            case 'percent':
                this.calculator.append('%');
                this.updateDisplay();
                break;
            case 'negate':
                this.wrapWithNegate();
                break;
            case 'reciprocal':
                this.calculator.append('^(-1)');
                this.updateDisplay();
                break;
            case 'mod':
                this.calculator.append(' mod ');
                this.updateDisplay();
                break;
            case 'abs':
                this.calculator.append('abs(');
                this.updateDisplay();
                break;
            case 'exp':
                this.calculator.append('e');
                this.updateDisplay();
                break;
            case 'rand':
                const rand = Math.random();
                this.calculator.append(rand.toString());
                this.updateDisplay();
                break;
            case 'ans':
                this.calculator.append('ans');
                this.updateDisplay();
                break;
        }
    }

    wrapWithNegate() {
        const expr = this.calculator.expression;
        if (expr && !expr.endsWith('(')) {
            // Find the last number or expression
            const match = expr.match(/(.*)\(([^()]+)\)$/);
            if (match) {
                this.calculator.expression = match[1] + '(-' + match[2] + ')';
            } else {
                this.calculator.expression = '-(' + expr + ')';
            }
            this.updateDisplay();
        }
    }

    calculate() {
        try {
            const result = this.calculator.calculate();
            this.updateDisplay();
            this.renderHistory();
        } catch (error) {
            this.resultEl.textContent = 'Error: ' + error.message;
        }
    }

    updateDisplay() {
        this.expressionEl.textContent = this.calculator.expression || '0';
        
        if (this.calculator.expression) {
            try {
                const preview = this.calculator.parser.parse(this.calculator.expression);
                this.resultEl.textContent = this.formatNumber(preview);
            } catch (e) {
                // Don't show preview if expression is incomplete
            }
        } else {
            this.resultEl.textContent = '0';
        }
    }

    formatNumber(num) {
        if (!isFinite(num)) return num.toString();
        if (Math.abs(num) < 1e-10 || Math.abs(num) >= 1e15) {
            return num.toExponential(8);
        }
        return parseFloat(num.toPrecision(12)).toString();
    }

    renderHistory() {
        const history = this.calculator.getHistory();
        this.historyList.innerHTML = history.map(item => `
            <div class="history-item" data-expression="${this.escapeHtml(item.expression)}">
                <div class="history-expression">${this.escapeHtml(item.expression)}</div>
                <div class="history-result">= ${this.formatNumber(item.result)}</div>
            </div>
        `).join('');

        // Add click handlers to history items
        this.historyList.querySelectorAll('.history-item').forEach(item => {
            item.addEventListener('click', () => {
                this.calculator.expression = item.dataset.expression;
                this.updateDisplay();
            });
        });
    }

    updateMemoryIndicator() {
        if (this.calculator.memory !== 0) {
            this.memoryIndicatorEl.classList.add('active');
        } else {
            this.memoryIndicatorEl.classList.remove('active');
        }
    }

    switchMode(mode) {
        this.modeTabs.forEach(tab => {
            tab.classList.toggle('active', tab.dataset.mode === mode);
        });

        this.modeContents.forEach(content => {
            content.classList.remove('active');
        });

        // Show appropriate content based on mode
        const contentMap = {
            'basic': 'basic-scientific-mode',
            'scientific': 'basic-scientific-mode',
            'complex': 'complex-mode-content',
            'matrix': 'matrix-mode-content',
            'vector': 'vector-mode-content',
            'stats': 'stats-mode-content',
            'probability': 'probability-mode-content',
            'calculus': 'calculus-mode-content',
            'solver': 'solver-mode-content',
            'converter': 'converter-mode-content',
            'financial': 'financial-mode-content',
            'programmer': 'programmer-mode-content'
        };

        const contentId = contentMap[mode];
        if (contentId) {
            document.getElementById(contentId).classList.add('active');
        }
    }

    toggleTheme() {
        this.theme = this.theme === 'dark' ? 'light' : 'dark';
        document.documentElement.setAttribute('data-theme', this.theme);
        this.themeToggle.textContent = this.theme === 'dark' ? '🌙' : '☀️';
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    handleKeyboard(e) {
        // Let keyboard handler process this
        if (window.keyboardHandler) {
            window.keyboardHandler.handleKey(e);
        }
    }

    exportHistory() {
        const history = this.calculator.getHistory();
        const json = JSON.stringify(history, null, 2);
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'calculator-history.json';
        a.click();
        URL.revokeObjectURL(url);
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = UIController;
}
