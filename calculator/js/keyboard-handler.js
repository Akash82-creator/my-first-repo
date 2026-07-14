/**
 * Keyboard Handler - Maps keyboard shortcuts to calculator functions
 */

class KeyboardHandler {
    constructor(calculator, ui) {
        this.calculator = calculator;
        this.ui = ui;
        
        this.keyMap = {
            // Numbers and basic operators
            '0': '0', '1': '1', '2': '2', '3': '3', '4': '4',
            '5': '5', '6': '6', '7': '7', '8': '8', '9': '9',
            '.': '.', ',': ',',
            '+': '+', '-': '-', '*': '×', '/': '÷',
            
            // Enter/Equals
            'Enter': 'equals',
            '=': 'equals',
            
            // Clear
            'Escape': 'clear-all',
            'Delete': 'clear-entry',
            'Backspace': 'backspace',
            
            // Parentheses
            '(': 'paren-open',
            ')': 'paren-close',
            
            // Functions (F-keys)
            'F1': 'deg-rad-grad',
            'F2': 'ln',
            'F3': 'log10',
            'F4': 'sin',
            'F5': 'cos',
            'F6': 'tan',
            'F7': 'sqrt',
            'F8': 'factorial',
            'F9': 'square',
            'F10': 'power',
            
            // Shortcuts for common operations
            's': 'sin',
            'c': 'cos',
            't': 'tan',
            'l': 'ln',
            'g': 'log10',
            'r': 'sqrt',
            'p': 'pi',
            'e': 'e',
            '^': 'power',
            '!': 'factorial',
            '%': 'percent',
            'a': 'abs',
            'n': 'negate',
            
            // Memory
            'm': 'ms',
            'M': 'mr',
        };
        
        this.bindEvents();
    }

    bindEvents() {
        document.addEventListener('keydown', (e) => this.handleKey(e));
    }

    handleKey(e) {
        // Ignore if typing in an input field
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
            return;
        }

        const key = e.key;
        const action = this.keyMap[key];

        if (action !== undefined) {
            e.preventDefault();
            
            if (action === 'equals') {
                this.ui.calculate();
            } else if (this.keyMap.hasOwnProperty(key) && !action.includes('-')) {
                // It's a direct value
                this.calculator.append(action);
                this.ui.updateDisplay();
            } else {
                // It's an action
                this.ui.handleAction(action);
            }
        }

        // Handle modifier keys
        if (e.ctrlKey || e.metaKey) {
            switch (key.toLowerCase()) {
                case 'h':
                    e.preventDefault();
                    this.ui.historyPanel.classList.toggle('collapsed');
                    break;
                case 't':
                    e.preventDefault();
                    this.ui.toggleTheme();
                    break;
                case 'e':
                    e.preventDefault();
                    this.ui.exportHistory();
                    break;
            }
        }

        // Shift modifiers
        if (e.shiftKey) {
            switch (key) {
                case '8': // Shift + 8 = *
                    e.preventDefault();
                    this.calculator.append('×');
                    this.ui.updateDisplay();
                    break;
            }
        }
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = KeyboardHandler;
}
