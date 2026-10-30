#!/usr/bin/env python3
"""Generate an independent 160-digit Taylor matrix-exponential reference."""

import argparse
import json
from decimal import Decimal, localcontext
from pathlib import Path

DESTINATION = Path(__file__).resolve().parents[2] / "fixtures/validation/ordered/matrices.json"


def multiply(a, b):
    n = len(a)
    return [[sum(a[i][k] * b[k][j] for k in range(n)) for j in range(n)] for i in range(n)]


def exponential(states, distance):
    with localcontext() as context:
        context.prec = 160
        rate = Decimal(states) / (2 * (states - 1))
        q = [[Decimal(0) for _ in range(states)] for _ in range(states)]
        for i in range(states):
            for j in [i - 1, i + 1]:
                if 0 <= j < states:
                    q[i][j] = rate
                    q[i][i] -= rate
        t = Decimal(str(distance))
        squares = 0
        while t > Decimal('0.125'):
            t /= 2
            squares += 1
        q = [[v * t for v in row] for row in q]
        result = [[Decimal(int(i == j)) for j in range(states)] for i in range(states)]
        term = [row[:] for row in result]
        for order in range(1, 500):
            product = multiply(term, q)
            term = [[value / order for value in row] for row in product]
            result = [[a + b for a, b in zip(row, update)] for row, update in zip(result, term)]
            if max(abs(v) for row in term for v in row) < Decimal('1e-150'):
                break
        else:
            raise ArithmeticError('Taylor series did not converge')
        for _ in range(squares):
            result = multiply(result, result)
        return [str(value) for row in result for value in row]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    cases = [dict(states=k, distance=t, matrix=exponential(k, t))
             for k in [2, 3, 4, 8] for t in [0, 1e-14, 1e-9, .001, .1, 1, 10, 10000, 1000000]]
    text = json.dumps(dict(precision_digits=160, method='Taylor series of Q with scaling and squaring',
                           cases=cases), indent=2) + '\n'
    if args.check:
        if DESTINATION.read_text() != text:
            raise SystemExit('Ordered matrix reference differs; regenerate the fixture')
    else:
        DESTINATION.parent.mkdir(exist_ok=True, parents=True)
        DESTINATION.write_text(text)
    print(f'{len(cases)} ordered matrix cases verified' if args.check else f'{len(cases)} cases written')


if __name__ == '__main__':
    main()
