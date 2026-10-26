import csv
import io
import unittest

from run import METRICS, read_results, redact_power_identifier, summarize


def output(samples=3):
    stream = io.StringIO()
    writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
    writer.writerow("case metric taxa characters patterns stride states categories shape missing_cells dirty_nodes baseline_log_likelihood allocation_calls reallocation_calls requested_bytes deallocation_calls sample iterations elapsed_ns".split())
    for metric in sorted(METRICS):
        for sample in range(samples):
            writer.writerow(["case", metric, 3, 4, 2, 8, 2, 1, "balanced", 0, 2,
                             -20, 0, 0, 0, 0, sample, 10, 100 + sample * 10])
    return stream.getvalue()


class ResultsTest(unittest.TestCase):
    def test_power_report_keeps_status_without_device_identifier(self):
        self.assertEqual(redact_power_identifier("AC Power\n -InternalBattery-0 (id=12345)\t60%; discharging"),
                         "AC Power\n -InternalBattery-0\t60%; discharging")

    def test_reports_medians_of_batch_means_without_counting_iterations_as_samples(self):
        first = read_results(output(), 3)
        second = read_results(output().replace("\t100\n", "\t200\n").replace("\t110\n", "\t220\n").replace("\t120\n", "\t240\n"), 3)
        result = summarize([first, second])["case"]["metrics"]["full_cached"]
        self.assertEqual(result["process_medians_ns"], [11, 22])
        self.assertEqual(result["median_ns"], 16.5)
        self.assertEqual(result["min_process_median_ns"], 11)
        self.assertEqual(result["max_process_median_ns"], 22)

    def test_rejects_truncated_or_repeated_samples(self):
        with self.assertRaises(ValueError):
            read_results(output(samples=2), 3)
        with self.assertRaises(ValueError):
            read_results(output() + output().splitlines()[-1] + "\n", 3)
        with self.assertRaises(ValueError):
            read_results("", 3)

    def test_rejects_changed_workloads_between_processes(self):
        first = read_results(output(), 3)
        changed = read_results(output().replace("\t-20\t", "\t-21\t"), 3)
        with self.assertRaises(ValueError):
            summarize([first, changed])


if __name__ == "__main__":
    unittest.main()
