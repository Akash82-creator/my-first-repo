/**
 * Financial Calculator - Compound interest, loans, investments
 */

class FinancialCalculator {
    // Compound Interest: A = P(1 + r/n)^(nt)
    static compoundInterest(principal, rate, time, compoundsPerYear = 12) {
        const r = rate / 100;
        const amount = principal * Math.pow(1 + r / compoundsPerYear, compoundsPerYear * time);
        return {
            amount,
            interest: amount - principal,
            breakdown: {
                principal,
                rate,
                time,
                compoundsPerYear
            }
        };
    }

    // Continuous Compounding: A = Pe^(rt)
    static continuousCompounding(principal, rate, time) {
        const r = rate / 100;
        const amount = principal * Math.exp(r * time);
        return {
            amount,
            interest: amount - principal
        };
    }

    // Loan EMI: E = P * r * (1+r)^n / ((1+r)^n - 1)
    static loanEMI(principal, annualRate, timeYears) {
        const r = annualRate / 100 / 12; // Monthly rate
        const n = timeYears * 12; // Number of months
        
        if (r === 0) {
            return {
                emi: principal / n,
                totalPayment: principal,
                totalInterest: 0
            };
        }
        
        const emi = principal * r * Math.pow(1 + r, n) / (Math.pow(1 + r, n) - 1);
        const totalPayment = emi * n;
        
        return {
            emi,
            totalPayment,
            totalInterest: totalPayment - principal,
            breakdown: {
                principal,
                annualRate,
                timeYears,
                monthlyRate: r * 100,
                months: n
            }
        };
    }

    // Amortization schedule
    static amortizationSchedule(principal, annualRate, timeYears) {
        const { emi } = FinancialCalculator.loanEMI(principal, annualRate, timeYears);
        const r = annualRate / 100 / 12;
        const n = timeYears * 12;
        
        const schedule = [];
        let balance = principal;
        
        for (let month = 1; month <= n; month++) {
            const interestPayment = balance * r;
            const principalPayment = emi - interestPayment;
            balance -= principalPayment;
            
            schedule.push({
                month,
                payment: emi,
                principal: principalPayment,
                interest: interestPayment,
                balance: Math.max(0, balance)
            });
        }
        
        return schedule;
    }

    // Future Value: FV = PV(1 + r)^t
    static futureValue(presentValue, rate, time) {
        const r = rate / 100;
        const fv = presentValue * Math.pow(1 + r, time);
        return {
            futureValue: fv,
            growth: fv - presentValue
        };
    }

    // Future Value with regular contributions
    static futureValueWithContributions(pv, contribution, rate, time, frequency = 12) {
        const r = rate / 100 / frequency;
        const n = time * frequency;
        
        const fvPrincipal = pv * Math.pow(1 + r, n);
        const fvContributions = contribution * (Math.pow(1 + r, n) - 1) / r;
        
        return {
            futureValue: fvPrincipal + fvContributions,
            fromPrincipal: fvPrincipal,
            fromContributions: fvContributions,
            totalContributions: contribution * n,
            totalInterest: fvPrincipal + fvContributions - pv - contribution * n
        };
    }

    // Present Value: PV = FV / (1 + r)^t
    static presentValue(futureValue, rate, time) {
        const r = rate / 100;
        return futureValue / Math.pow(1 + r, time);
    }

    // Present Value of annuity
    static presentValueOfAnnuity(payment, rate, periods) {
        const r = rate / 100;
        return payment * (1 - Math.pow(1 + r, -periods)) / r;
    }

    // CAGR: (End/Start)^(1/t) - 1
    static cagr(startValue, endValue, time) {
        return (Math.pow(endValue / startValue, 1 / time) - 1) * 100;
    }

    // ROI: (Gain - Cost) / Cost * 100
    static roi(gain, cost) {
        return ((gain - cost) / cost) * 100;
    }

    // Depreciation (Straight-line)
    static straightLineDepreciation(cost, salvageValue, life) {
        const annualDepreciation = (cost - salvageValue) / life;
        return {
            annualDepreciation,
            rate: (annualDepreciation / cost) * 100
        };
    }

    // Depreciation (Declining balance)
    static decliningBalanceDepreciation(cost, salvageValue, life, factor = 2) {
        const rate = factor / life;
        const schedule = [];
        let bookValue = cost;
        
        for (let year = 1; year <= life; year++) {
            const depreciation = bookValue * rate;
            bookValue -= depreciation;
            
            schedule.push({
                year,
                depreciation: Math.min(depreciation, bookValue + depreciation - salvageValue),
                bookValue: Math.max(bookValue, salvageValue)
            });
        }
        
        return schedule;
    }

    // Net Present Value
    static npv(initialInvestment, cashFlows, discountRate) {
        const r = discountRate / 100;
        let npv = -initialInvestment;
        
        for (let t = 0; t < cashFlows.length; t++) {
            npv += cashFlows[t] / Math.pow(1 + r, t + 1);
        }
        
        return npv;
    }

    // Internal Rate of Return (using Newton-Raphson)
    static irr(initialInvestment, cashFlows, guess = 10) {
        const maxIter = 100;
        const tolerance = 1e-6;
        let r = guess / 100;
        
        for (let i = 0; i < maxIter; i++) {
            let npv = -initialInvestment;
            let dNpv = 0;
            
            for (let t = 0; t < cashFlows.length; t++) {
                npv += cashFlows[t] / Math.pow(1 + r, t + 1);
                dNpv -= (t + 1) * cashFlows[t] / Math.pow(1 + r, t + 2);
            }
            
            const newR = r - npv / dNpv;
            if (Math.abs(newR - r) < tolerance) {
                return newR * 100;
            }
            r = newR;
        }
        
        return r * 100;
    }

    // Payback Period
    static paybackPeriod(initialInvestment, cashFlows) {
        let cumulative = -initialInvestment;
        
        for (let t = 0; t < cashFlows.length; t++) {
            cumulative += cashFlows[t];
            if (cumulative >= 0) {
                const prevCumulative = cumulative - cashFlows[t];
                return t + (-prevCumulative) / cashFlows[t];
            }
        }
        
        return Infinity; // Never pays back
    }

    // Profitability Index
    static profitabilityIndex(initialInvestment, cashFlows, discountRate) {
        const pvCashFlows = cashFlows.reduce((sum, cf, t) => 
            sum + cf / Math.pow(1 + discountRate / 100, t + 1), 0);
        return pvCashFlows / initialInvestment;
    }
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = FinancialCalculator;
}
