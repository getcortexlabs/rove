package rove.reference;

import beast.base.core.BEASTInterface;
import beast.base.evolution.alignment.Alignment;
import beast.base.evolution.likelihood.TreeLikelihood;
import beast.base.evolution.sitemodel.SiteModel;
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

    private static JSONObject summarize(String log, String trees, String truth, int burnin) throws Exception {
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
            }
            for (String clade : expected) {
                indicators.get(clade).add(observed.contains(clade) ? 1.0 : 0.0);
            }
            Set<String> difference = new TreeSet<>(observed);
            difference.removeAll(expected);
            totalDistance += difference.size();
            difference = new TreeSet<>(expected);
            difference.removeAll(observed);
            totalDistance += difference.size();
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
        return new JSONObject()
                .put("burnin_percent", burnin)
                .put("discarded_samples", skip)
                .put("retained_samples", retained)
                .put("statistics", statistics)
                .put("reference_clades", supports)
                .put("observed_clade_frequencies", frequencies)
                .put("unique_topologies", topologies.size())
                .put("reference_topology_posterior", topologies.getOrDefault(String.join(";", expected), 0) / (double) retained)
                .put("mean_rooted_rf_distance_to_reference", totalDistance / retained);
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
            patterns.put(new JSONObject()
                    .put("states", new JSONArray(data.getPattern(index)))
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

    public static void main(String[] args) throws Exception {
        PackageManager.loadExternalJars();
        JSONObject result;
        switch (args[0]) {
            case "summary" -> result = summarize(args[1], args[2], args[3], Integer.parseInt(args[4]));
            case "snapshot" -> result = snapshot(args[1], args[2]);
            case "self-test" -> {
                selfTest();
                return;
            }
            default -> throw new IllegalArgumentException("Expected summary, snapshot, or self-test");
        }
        Files.writeString(Path.of(args[args.length - 1]), result.toString(2) + "\n");
    }
}
