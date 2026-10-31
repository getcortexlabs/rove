import math
import random
import unittest

from marginal_likelihood import estimate, ess, schedule


def gaussian_samples(tau, powers, draws, seed):
    generator = random.Random(seed)
    samples = []
    for power in powers:
        sd = 1.0 / math.sqrt(1.0 + power * tau)
        samples.append((power, [-tau * (sd * generator.gauss(0.0, 1.0)) ** 2 / 2 for _ in range(draws)]))
    return samples


class Estimators(unittest.TestCase):
    def test_schedule_is_beta_quantiles(self):
        self.assertEqual(schedule(5, 0.5), [0.0, 0.0625, 0.25, 0.5625, 1.0])

    def test_constant_likelihood_is_exact(self):
        samples = [(p, [-3.5] * 40) for p in schedule(6, 0.3)]
        result = estimate(samples)
        self.assertAlmostEqual(result["stepping_stone"]["log_marginal_likelihood"], -3.5, places=12)
        self.assertAlmostEqual(result["path_sampling"]["log_marginal_likelihood"], -3.5, places=12)
        self.assertEqual(result["stepping_stone"]["standard_error"], 0.0)

    def test_gaussian_evidence_within_reported_error(self):
        tau = 9.0
        expected = -0.5 * math.log(1.0 + tau)
        within = 0
        for seed in range(12):
            result = estimate(gaussian_samples(tau, schedule(16, 0.3), 1500, seed))
            for method in ("stepping_stone", "path_sampling"):
                z = (result[method]["log_marginal_likelihood"] - expected) / result[method]["standard_error"]
                self.assertLess(abs(z), 5.0, (seed, method))
                within += abs(z) < 2.0
        self.assertGreaterEqual(within, 18)

    def test_ess_matches_independent_and_autocorrelated_series(self):
        generator = random.Random(3)
        independent = [generator.gauss(0, 1) for _ in range(4000)]
        effective, _ = ess(independent)
        self.assertGreater(effective, 3000)
        correlated, value = [], 0.0
        for _ in range(4000):
            value = 0.9 * value + generator.gauss(0, 1)
            correlated.append(value)
        effective, _ = ess(correlated)
        self.assertLess(effective, 600)
        self.assertEqual(ess([1.0] * 10)[0], None)

    def test_powers_must_cover_the_unit_interval(self):
        with self.assertRaises(ValueError):
            estimate([(0.0, [1.0, 2.0]), (0.5, [1.0, 2.0])])
        with self.assertRaises(ValueError):
            estimate([(0.1, [1.0, 2.0]), (1.0, [1.0, 2.0])])


if __name__ == "__main__":
    unittest.main()
