package rove.reference;

import beast.base.evolution.likelihood.BeerLikelihoodCore;
import beast.base.evolution.likelihood.BeerLikelihoodCore4;
import beagle.Beagle;
import beagle.BeagleFactory;
import beagle.BeagleFlag;
import org.json.JSONArray;
import org.json.JSONObject;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;

/** Optional adapters for the numerical workloads exported by benches/likelihood.rs. */
public final class KernelBenchmark {
    private static volatile double sink;

    private static double[] doubles(JSONArray values) throws Exception {
        double[] result = new double[values.length()];
        for (int i = 0; i < result.length; i++) result[i] = values.getDouble(i);
        return result;
    }

    private static final class Engine implements AutoCloseable {
        final JSONObject input;
        final int k, patterns, tips, nodes, root, categories, observed, characters, tipBranch, internalBranch;
        final boolean variable;
        final int[][] children;
        final int[] parents, operations, scaleIndices, rootIndex, zero = {0}, scaleIndex;
        final double[] distances, rates, frequencies, categoryWeights, weights;
        final double[] matrix, matrices, integrated, logs, total = new double[1];
        final BeerLikelihoodCore beer;
        final Beagle beagle;
        final JSONObject implementation;
        boolean alternate;

        Engine(JSONObject input, String backend) throws Exception {
            this.input = input;
            observed = input.getInt("observed_patterns");
            characters = input.getInt("characters");
            tipBranch = input.getInt("tip_branch");
            internalBranch = input.getInt("internal_branch");
            variable = input.getBoolean("variable");
            k = input.getInt("states");
            patterns = input.getInt("core_patterns");
            tips = input.getInt("taxa");
            nodes = 2 * tips - 1;
            root = nodes - 1;
            rootIndex = new int[]{root};
            scaleIndex = new int[]{nodes};
            rates = doubles(input.getJSONArray("rates"));
            categories = rates.length;
            distances = doubles(input.getJSONArray("distances"));
            frequencies = doubles(input.getJSONArray("frequencies"));
            categoryWeights = doubles(input.getJSONArray("category_weights"));
            weights = Arrays.copyOf(doubles(input.getJSONArray("weights")), patterns);
            matrix = new double[k * k];
            matrices = new double[categories * k * k];
            integrated = new double[patterns * k];
            logs = new double[patterns];
            children = new int[nodes][];
            parents = new int[nodes];
            Arrays.fill(parents, -1);
            operations = new int[(tips - 1) * 7];
            scaleIndices = new int[tips - 1];
            JSONArray pairs = input.getJSONArray("children");
            for (int parent = tips; parent < nodes; parent++) {
                JSONArray pair = pairs.getJSONArray(parent);
                int a = pair.getInt(0), b = pair.getInt(1);
                children[parent] = new int[]{a, b};
                parents[a] = parents[b] = parent;
                scaleIndices[parent - tips] = parent;
            }
            if (backend.equals("java")) {
                beer = k == 4 ? new BeerLikelihoodCore4() : new BeerLikelihoodCore(k);
                beagle = null;
                beer.initialize(nodes, patterns, categories, true, true);
                beer.setUseScaling(1.01);
                for (int node = tips; node < nodes; node++) beer.createNodePartials(node);
                implementation = new JSONObject().put("class", beer.getClass().getName())
                    .put("scaling", "BeerLikelihoodCore scaling enabled; threshold-based rescaling");
            } else {
                beer = null;
                long required = BeagleFlag.PRECISION_DOUBLE.getMask() | BeagleFlag.PROCESSOR_CPU.getMask()
                    | BeagleFlag.THREADING_NONE.getMask() | BeagleFlag.COMPUTATION_SYNCH.getMask()
                    | BeagleFlag.SCALING_MANUAL.getMask() | BeagleFlag.SCALERS_LOG.getMask();
                beagle = BeagleFactory.loadBeagleInstance(tips, nodes, 0, k, patterns, 1, nodes,
                    categories, nodes + 1, new int[]{0}, BeagleFlag.VECTOR_SSE.getMask(), required);
                var details = beagle.getDetails();
                if (details == null || !beagle.getClass().getName().equals("beagle.BeagleJNIImpl")
                    || (details.getFlags() & required) != required) {
                    throw new IllegalStateException("BEAGLE requirements were not met");
                }
                beagle.setStateFrequencies(0, frequencies);
                beagle.setCategoryWeights(0, categoryWeights);
                beagle.setPatternWeights(weights);
                implementation = new JSONObject().put("class", beagle.getClass().getName())
                    .put("version", BeagleFactory.getVersion()).put("resource", details.getResourceName())
                    .put("implementation", details.getImplementationName())
                    .put("flags", BeagleFlag.toString(details.getFlags()))
                    .put("scaling", "manual; write every updated node, accumulate all internal scalers");
            }
            JSONArray partials = input.getJSONArray("tip_partials");
            for (int tip = 0; tip < tips; tip++) {
                double[] values = doubles(partials.getJSONArray(tip));
                if (beer != null) beer.setNodePartials(tip, values);
                else beagle.setTipPartials(tip, values);
            }
            for (int node = 0; node < root; node++) setBranch(node, distances[node]);
        }

        void setBranch(int node, double distance) {
            for (int c = 0; c < categories; c++) {
                double x = -distance * rates[c] * k / (k - 1.0);
                double stay = Math.exp(x), change = -Math.expm1(x);
                for (int i = 0; i < k; i++) {
                    for (int j = 0; j < k; j++) matrix[i * k + j] = frequencies[j] * change + (i == j ? stay : 0);
                }
                if (beer != null) beer.setNodeMatrix(node, c, matrix);
                else System.arraycopy(matrix, 0, matrices, c * k * k, matrix.length);
            }
            if (beagle != null) beagle.setTransitionMatrix(node, matrices, 1.0);
        }

        int updateNode(int node, int count) {
            int a = children[node][0], b = children[node][1];
            if (beer != null) beer.calculatePartials(a, b, node);
            else {
                int offset = count * 7;
                operations[offset] = node;
                operations[offset + 1] = node;
                operations[offset + 2] = Beagle.NONE;
                operations[offset + 3] = a;
                operations[offset + 4] = a;
                operations[offset + 5] = b;
                operations[offset + 6] = b;
            }
            return count + 1;
        }

        double score(int count) {
            if (beer != null) {
                beer.integratePartials(root, categoryWeights, integrated);
                beer.calculateLogLikelihoods(integrated, frequencies, logs);
            } else {
                beagle.updatePartials(operations, count, Beagle.NONE);
                beagle.resetScaleFactors(nodes);
                beagle.accumulateScaleFactors(scaleIndices, scaleIndices.length, nodes);
                beagle.calculateRootLogLikelihoods(rootIndex, zero, zero, scaleIndex, 1, total);
                beagle.getSiteLogLikelihoods(logs);
            }
            double result = 0;
            for (int i = 0; i < observed; i++) result += logs[i] * weights[i];
            if (variable) {
                double maximum = Double.NEGATIVE_INFINITY, sum = 0;
                for (int i = patterns - k; i < patterns; i++) maximum = Math.max(maximum, logs[i]);
                for (int i = patterns - k; i < patterns; i++) sum += Math.exp(logs[i] - maximum);
                double constantLogP = maximum + Math.log(sum);
                result -= characters * Math.log(-Math.expm1(constantLogP));
            }
            return result;
        }

        double full() {
            int count = 0;
            for (int node = tips; node < nodes; node++) count = updateNode(node, count);
            return score(count);
        }

        double run(String metric) {
            if (metric.equals("full_cached")) return full();
            alternate = !alternate;
            double factor = alternate ? 1.1 : 0.9;
            if (metric.equals("full_with_branches")) {
                for (int node = 0; node < root; node++) setBranch(node, distances[node] * factor);
                return full();
            }
            int branch = metric.equals("tip_update") ? tipBranch : internalBranch;
            setBranch(branch, distances[branch] * factor);
            int count = 0;
            for (int node = parents[branch]; node != -1; node = parents[node]) count = updateNode(node, count);
            return score(count);
        }

        public void close() throws Exception {
            if (beagle != null) {
                try { beagle.finalize(); }
                catch (Throwable failure) { throw new Exception(failure); }
            }
        }
    }

    private static void close(double value, double expected) {
        if (!Double.isFinite(value) || Math.abs(value - expected) > 1e-8)
            throw new IllegalStateException("Likelihood differs: " + value + " vs " + expected);
    }

    private static long batch(Engine engine, String metric, int iterations) {
        long start = System.nanoTime();
        double value = 0;
        for (int i = 0; i < iterations; i++) value += engine.run(metric);
        long elapsed = System.nanoTime() - start;
        sink = value;
        return elapsed;
    }

    public static void main(String[] args) throws Exception {
        // input, backend, output, warmup milliseconds, samples, sample milliseconds
        JSONArray inputs = new JSONArray(Files.readString(Path.of(args[0])));
        String backend = args[1];
        if (!backend.equals("java") && !backend.equals("beagle")) throw new IllegalArgumentException("Unknown backend");
        int warmup = Integer.parseInt(args[3]), samples = Integer.parseInt(args[4]), sampleMs = Integer.parseInt(args[5]);
        if (warmup < 0 || samples < 0 || sampleMs <= 0) throw new IllegalArgumentException("Invalid timing parameters");
        JSONArray results = new JSONArray();
        for (int i = 0; i < inputs.length(); i++) {
            JSONObject input = inputs.getJSONObject(i);
            for (String metric : new String[]{"full_cached", "full_with_branches", "tip_update", "internal_update"}) {
                long setup = System.nanoTime();
                try (Engine engine = new Engine(input, backend)) {
                    setup = System.nanoTime() - setup;
                    JSONObject check = input.getJSONObject("checks").getJSONObject(metric);
                    double initial = engine.full();
                    close(initial, check.getDouble("initial"));
                    JSONArray values = check.getJSONArray("values");
                    for (int j = 0; j < values.length(); j++) close(engine.run(metric), values.getDouble(j));
                    long start = System.nanoTime();
                    while (System.nanoTime() - start < warmup * 1_000_000L) batch(engine, metric, 8);
                    int iterations = 1;
                    if (samples > 0) {
                        while (batch(engine, metric, iterations) < sampleMs * 1_000_000L)
                            iterations = Math.multiplyExact(iterations, 2);
                    }
                    JSONArray elapsed = new JSONArray();
                    for (int j = 0; j < samples; j++) elapsed.put(batch(engine, metric, iterations));
                    results.put(new JSONObject().put("case", input.getString("name")).put("metric", metric)
                        .put("backend", backend).put("implementation", engine.implementation)
                        .put("initial", initial).put("setup_ns", setup).put("iterations", iterations)
                        .put("elapsed_ns", elapsed));
                }
            }
        }
        Files.writeString(Path.of(args[2]), results.toString(2) + "\n");
    }
}
