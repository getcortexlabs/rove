package rove.reference;

import beast.base.core.BEASTInterface;
import beast.base.evolution.alignment.Alignment;
import beast.base.evolution.branchratemodel.UCRelaxedClockModel;
import beast.base.evolution.likelihood.TreeLikelihood;
import beast.base.evolution.sitemodel.SiteModel;
import beast.base.evolution.substitutionmodel.JukesCantor;
import beast.base.inference.distribution.Gamma;
import beast.base.inference.distribution.LogNormalDistributionModel;
import beast.base.evolution.tree.Node;
import beast.base.evolution.tree.Tree;
import beast.base.evolution.tree.TreeParser;
import beast.base.inference.Distribution;
import beast.base.inference.MCMC;
import beast.base.inference.State;
import beast.base.inference.StateNode;
import beast.base.inference.parameter.RealParameter;
import beast.base.inference.util.ESS;
import beast.base.parser.NexusParser;
import beast.base.parser.XMLParser;
import beast.pkgmgmt.PackageManager;
import beastfx.app.tools.LogAnalyser;
import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;

/** Reference exports using the pinned BEAST 2 APIs, outside the Rust runtime. */
public final class ReferenceDiagnostics {
    private static Object finite(double value) {
        return Double.isFinite(value) ? value : JSONObject.NULL;
    }

    private static Set<String> collectClades(Node node, Set<String> clades) {
        Set<String> taxa = new TreeSet<>();
        if (node.isLeaf()) {
            taxa.add(node.getID());
        } else {
            for (Node child : node.getChildren()) {
                taxa.addAll(collectClades(child, clades));
            }
            if (!node.isRoot()) {
                clades.add(String.join(",", taxa));
            }
        }
        return taxa;
    }

    private static Set<String> clades(Tree tree) {
        Set<String> result = new TreeSet<>();
        collectClades(tree.getRoot(), result);
        return result;
    }

    private static JSONObject summarize(String log, String trees, String truth, int burnin, boolean extended) throws Exception {
        if (burnin < 0 || burnin >= 100) {
            throw new IllegalArgumentException("Burn-in must be between 0 and 99 percent");
        }
        LogAnalyser analyser = new LogAnalyser(log, burnin, true);
        JSONObject statistics = new JSONObject();
        for (String label : analyser.getLabels()) {
            statistics.put(label, new JSONObject()
                    .put("mean", finite(analyser.getMean(label)))
                    .put("sd", finite(analyser.getStdDev(label)))
                    .put("ess", finite(analyser.getESS(label)))
                    .put("hpd_low", finite(analyser.get95HPDlow(label)))
                    .put("hpd_high", finite(analyser.get95HPDup(label))));
        }
        NexusParser parser = new NexusParser();
        parser.parseFile(new File(trees));
        int skip = parser.trees.size() * burnin / 100;
        int retained = parser.trees.size() - skip;
        if (retained != analyser.getTrace(0).length) {
            throw new IllegalArgumentException("Tree and parameter sample counts differ");
        }
        Tree expectedTree = new TreeParser(Files.readString(Path.of(truth)).trim());
        Set<String> expected = clades(expectedTree);
        Set<String> expectedTaxa = collectClades(expectedTree.getRoot(), new TreeSet<>());
        Map<String, Integer> counts = new TreeMap<>();
        Map<String, Integer> topologies = new HashMap<>();
        Map<String, List<Double>> indicators = new TreeMap<>();
        Map<String, int[]> batches = new TreeMap<>();
        int batchSize = retained / 20;
        List<Double> distances = new ArrayList<>();
        List<Double> exactTopologies = new ArrayList<>();
        for (String clade : expected) {
            indicators.put(clade, new ArrayList<>());
        }
        double totalDistance = 0;
        for (int index = skip; index < parser.trees.size(); index++) {
            Tree tree = parser.trees.get(index);
            Set<String> observed = new TreeSet<>();
            if (!collectClades(tree.getRoot(), observed).equals(expectedTaxa)) {
                throw new IllegalArgumentException("Tree taxon labels differ from the reference");
            }
            for (String clade : observed) {
                counts.merge(clade, 1, Integer::sum);
                if (extended && batchSize > 0 && index - skip < 20 * batchSize) {
                    batches.computeIfAbsent(clade, key -> new int[20])[(index - skip) / batchSize]++;
                }
            }
            for (String clade : expected) {
                indicators.get(clade).add(observed.contains(clade) ? 1.0 : 0.0);
            }
            Set<String> difference = new TreeSet<>(observed);
            difference.removeAll(expected);
            int distance = difference.size();
            difference = new TreeSet<>(expected);
            difference.removeAll(observed);
            distance += difference.size();
            totalDistance += distance;
            if (extended) {
                distances.add((double) distance);
                exactTopologies.add(distance == 0 ? 1.0 : 0.0);
            }
            topologies.merge(String.join(";", observed), 1, Integer::sum);
        }
        JSONObject supports = new JSONObject();
        for (String clade : expected) {
            double probability = counts.getOrDefault(clade, 0) / (double) retained;
            boolean constant = probability == 0 || probability == 1;
            supports.put(clade, new JSONObject()
                    .put("posterior", probability)
                    .put("constant_trace", constant)
                    .put("ess", constant ? JSONObject.NULL : finite(ESS.calcESS(indicators.get(clade)))));
        }
        JSONObject frequencies = new JSONObject();
        for (Map.Entry<String, Integer> entry : counts.entrySet()) {
            frequencies.put(entry.getKey(), entry.getValue() / (double) retained);
        }
        JSONObject result = new JSONObject()
                .put("burnin_percent", burnin)
                .put("discarded_samples", skip)
                .put("retained_samples", retained)
                .put("statistics", statistics)
                .put("reference_clades", supports)
                .put("observed_clade_frequencies", frequencies)
                .put("unique_topologies", topologies.size())
                .put("reference_topology_posterior", topologies.getOrDefault(String.join(";", expected), 0) / (double) retained)
                .put("mean_rooted_rf_distance_to_reference", totalDistance / retained);
        if (extended) {
            JSONObject batchFrequencies = new JSONObject();
            for (Map.Entry<String, int[]> entry : batches.entrySet()) {
                JSONArray values = new JSONArray();
                for (int count : entry.getValue()) {
                    values.put(count / (double) batchSize);
                }
                batchFrequencies.put(entry.getKey(), values);
            }
            result.put("clade_batch_frequencies", batchFrequencies)
                    .put("clade_batch_size", batchSize)
                    .put("clade_batch_omitted_samples", retained - 20 * batchSize)
                    .put("tree_statistics", new JSONObject()
                            .put("rooted_rf", moments(distances))
                            .put("reference_topology", moments(exactTopologies)))
                    .put("tree_traces", new JSONObject()
                            .put("rooted_rf", new JSONArray(distances))
                            .put("reference_topology", new JSONArray(exactTopologies)));
        }
        return result;
    }

    private static JSONObject moments(List<Double> values) throws Exception {
        double mean = values.stream().mapToDouble(Double::doubleValue).average().orElseThrow();
        double variance = values.stream().mapToDouble(x -> (x - mean) * (x - mean)).sum() / (values.size() - 1);
        return new JSONObject().put("mean", mean).put("sd", Math.sqrt(variance))
                .put("ess", variance == 0 ? JSONObject.NULL : finite(ESS.calcESS(values)));
    }

    private static void visit(BEASTInterface object, Set<BEASTInterface> seen, Map<String, BEASTInterface> objects) throws Exception {
        if (!seen.add(object)) {
            return;
        }
        if (object.getID() != null) {
            objects.put(object.getID(), object);
        }
        for (BEASTInterface child : object.listActiveBEASTObjects()) {
            visit(child, seen, objects);
        }
    }

    private static JSONObject snapshot(String xml, String checkpoint) throws Exception {
        MCMC mcmc = (MCMC) new XMLParser().parseFile(new File(xml));
        State state = mcmc.startStateInput.get();
        state.setStateFileName(checkpoint);
        state.restoreFromFile();
        double posterior = state.robustlyCalcPosterior(mcmc.posteriorInput.get());
        Map<String, BEASTInterface> objects = new TreeMap<>();
        visit(mcmc, Collections.newSetFromMap(new IdentityHashMap<>()), objects);
        JSONObject contributions = new JSONObject();
        TreeLikelihood likelihood = null;
        for (BEASTInterface object : objects.values()) {
            if (object instanceof Distribution distribution) {
                contributions.put(object.getID(), finite(distribution.getCurrentLogP()));
            }
            if (object instanceof TreeLikelihood treeLikelihood) {
                if (likelihood != null) {
                    throw new IllegalArgumentException("This exporter supports one tree likelihood");
                }
                likelihood = treeLikelihood;
            }
        }
        if (likelihood == null || !Double.isFinite(posterior)) {
            throw new IllegalStateException("No finite tree likelihood found");
        }
        Tree tree = (Tree) likelihood.treeInput.get();
        SiteModel siteModel = (SiteModel) likelihood.siteModelInput.get();
        Alignment data = likelihood.dataInput.get();
        JSONArray nodes = new JSONArray();
        for (Node node : tree.getNodesAsArray()) {
            JSONArray children = new JSONArray();
            for (Node child : node.getChildren()) {
                children.put(child.getNr());
            }
            JSONObject record = new JSONObject()
                    .put("index", node.getNr())
                    .put("height", node.getHeight())
                    .put("children", children);
            if (node.isLeaf()) {
                record.put("taxon", node.getID());
            }
            if (!node.isRoot()) {
                record.put("branch_rate", likelihood.branchRateModelInput.get().getRateForBranch(node));
            }
            nodes.put(record);
        }
        double[] patternLogs = likelihood.getPatternLogLikelihoods();
        double correction = data.isAscertained ? data.getAscertainmentCorrection(patternLogs) : 0;
        double reconstructed = 0;
        JSONArray patterns = new JSONArray();
        for (int index = 0; index < data.getPatternCount(); index++) {
            int weight = data.getPatternWeight(index);
            reconstructed += weight * (patternLogs[index] - correction);
            JSONArray stateSets = new JSONArray();
            for (int code : data.getPattern(index)) {
                stateSets.put(new JSONArray(data.getStateSet(code)));
            }
            patterns.put(new JSONObject()
                    .put("states", new JSONArray(data.getPattern(index)))
                    .put("state_sets", stateSets)
                    .put("weight", weight)
                    .put("log_likelihood", patternLogs[index]));
        }
        if (Math.abs(reconstructed - likelihood.getCurrentLogP()) > 1e-9) {
            throw new IllegalStateException("Pattern likelihoods do not reconstruct the tree likelihood");
        }
        JSONObject parameters = new JSONObject();
        for (StateNode node : state.stateNodeInput.get()) {
            if (node instanceof RealParameter parameter) {
                parameters.put(node.getID(), new JSONArray(parameter.getValues()));
            }
        }
        return new JSONObject()
                .put("posterior", posterior)
                .put("distributions", contributions)
                .put("parameters", parameters)
                .put("taxa", new JSONArray(data.getTaxaNames()))
                .put("root", tree.getRoot().getNr())
                .put("nodes", nodes)
                .put("frequencies", new JSONArray(likelihood.getSubstitutionModel().getFrequencies()))
                .put("category_rates", new JSONArray(siteModel.getCategoryRates(tree.getRoot())))
                .put("category_weights", new JSONArray(siteModel.getCategoryProportions(tree.getRoot())))
                .put("patterns", patterns)
                .put("log_ascertainment_correction", correction)
                .put("reconstructed_log_likelihood", reconstructed);
    }

    private static void selfTest() throws Exception {
        Tree tree = new TreeParser("((T1,T2),(T3,T4));");
        if (!clades(tree).equals(Set.of("T1,T2", "T3,T4"))) {
            throw new AssertionError("Incorrect rooted clades");
        }
        Path file = Files.createTempFile("rove-translated-tree-", ".nex");
        try {
            Files.writeString(file, "#NEXUS\nBegin trees;\nTranslate 1 T3, 2 T1, 3 T4, 4 T2;\ntree STATE_0 = ((2:1,4:1):1,(1:1,3:1):1);\nEnd;\n");
            NexusParser parser = new NexusParser();
            parser.parseFile(file.toFile());
            if (!clades(parser.trees.get(0)).equals(clades(tree))) {
                throw new AssertionError("NEXUS translation changed taxon identities");
            }
        } finally {
            Files.deleteIfExists(file);
        }
        System.out.println("Clade extraction and NEXUS translation checks passed");
    }

    /** Numerical checks for the rate model, independent of a chain's state. */
    private static JSONObject rateModel() throws Exception {
        JSONArray categories = new JSONArray();
        // Include both support boundaries, small-shape underflow, and the two
        // archived states. JukesCantor is only required to initialize SiteModel;
        // the substitution model does not enter the gamma category calculation.
        double[] shapes = {0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5,
                0.810814173511043, 1, 2, 3.006475849547493, 5, 10, 20, 50, 100, 500, 1000};
        for (double shape : shapes) {
            SiteModel model = new SiteModel();
            RealParameter parameter = new RealParameter();
            parameter.initByName("value", Double.toString(shape));
            model.initByName("substModel", new JukesCantor(), "gammaCategoryCount", 4,
                    "shape", parameter);
            categories.put(new JSONObject().put("shape", shape)
                    .put("lower", parameter.getLower()).put("upper", parameter.getUpper())
                    .put("rates", new JSONArray(model.getCategoryRates(null)))
                    .put("weights", new JSONArray(model.getCategoryProportions(null))));
        }
        JSONArray priors = new JSONArray();
        for (double sigma : new double[]{0.01, 0.2, 0.5142003313329568, 1, 5, 20}) {
            LogNormalDistributionModel lognormal = new LogNormalDistributionModel();
            lognormal.initByName("M", new RealParameter("1"), "S",
                    new RealParameter(Double.toString(sigma)), "meanInRealSpace", true);
            Gamma gamma = new Gamma();
            gamma.initByName("alpha", new RealParameter("5"), "beta", new RealParameter("0.2"));
            JSONArray densities = new JSONArray();
            for (double rate : new double[]{1e-100, 0.25, 0.5, 1, 2, 10}) {
                densities.put(new JSONObject().put("rate", rate).put("log_density", lognormal.logDensity(rate)));
            }
            priors.put(new JSONObject().put("sigma", sigma).put("sigma_log_density", gamma.logDensity(sigma))
                    .put("rate_densities", densities));
        }
        JSONArray mappings = new JSONArray();
        for (int rootIndex : new int[]{3, 4}) {
            Tree tree = new TreeParser("((A:1,B:1):1,C:2);");
            Node root = tree.getRoot();
            Node other = tree.getNode(rootIndex);
            other.setNr(root.getNr());
            root.setNr(rootIndex);
            tree.initArrays();
            RealParameter rates = new RealParameter();
            rates.initByName("value", "0.25 0.5 1.5 2", "lower", 1e-100);
            LogNormalDistributionModel distribution = new LogNormalDistributionModel();
            distribution.initByName("M", new RealParameter("1"), "S", new RealParameter("0.2"), "meanInRealSpace", true);
            UCRelaxedClockModel clock = new UCRelaxedClockModel();
            clock.initByName("tree", tree, "rates", rates, "distr", distribution);
            JSONArray branches = new JSONArray();
            for (Node node : tree.getNodesAsArray()) {
                branches.put(clock.getRateForBranch(node));
            }
            mappings.put(new JSONObject().put("root", rootIndex).put("parameters", new JSONArray(rates.getValues()))
                    .put("branch_rates", branches).put("lower_after_init", rates.getLower()));
        }
        return new JSONObject().put("gamma_categories", categories).put("clock_priors", priors)
                .put("branch_mapping", mappings);
    }

    public static void main(String[] args) throws Exception {
        PackageManager.loadExternalJars();
        JSONObject result;
        switch (args[0]) {
            case "summary" -> result = summarize(args[1], args[2], args[3], Integer.parseInt(args[4]), false);
            case "inference-summary" -> result = summarize(args[1], args[2], args[3], Integer.parseInt(args[4]), true);
            case "snapshot" -> result = snapshot(args[1], args[2]);
            case "rate-model" -> result = rateModel();
            case "self-test" -> {
                selfTest();
                return;
            }
            default -> throw new IllegalArgumentException("Expected summary, inference-summary, snapshot, rate-model, or self-test");
        }
        Files.writeString(Path.of(args[args.length - 1]), result.toString(2) + "\n");
    }
}
