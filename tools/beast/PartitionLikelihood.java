package rove.reference;

import beast.base.evolution.alignment.Alignment;
import beast.base.evolution.alignment.Sequence;
import beast.base.evolution.branchratemodel.BranchRateModel;
import beast.base.evolution.branchratemodel.StrictClockModel;
import beast.base.evolution.branchratemodel.UCRelaxedClockModel;
import beast.base.evolution.datatype.StandardData;
import beast.base.evolution.likelihood.TreeLikelihood;
import beast.base.evolution.sitemodel.SiteModel;
import beast.base.evolution.tree.Node;
import beast.base.evolution.tree.Tree;
import beast.base.inference.distribution.LogNormalDistributionModel;
import beast.base.inference.parameter.RealParameter;
import beast.base.parser.NexusParser;
import beast.pkgmgmt.PackageManager;
import morphmodels.evolution.substitutionmodel.LewisMK;
import morphmodels.evolution.substitutionmodel.Ordinal;
import beast.base.evolution.substitutionmodel.Frequencies;
import beast.base.evolution.substitutionmodel.SubstitutionModel;
import java.util.Set;
import java.util.HashSet;
import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.TreeSet;

/** Fixed-state partition oracle using BEAST 2.7.8 and MM 1.2.1. */
public final class PartitionLikelihood {
    private static final double[] HEIGHTS = {0, 0, 0, 0, 0, 0, .12, .18, .2, .43, .7};
    private static final int[][] CHILDREN = {{0, 1}, {2, 3}, {4, 5}, {6, 7}, {9, 8}};
    private static final double[] RATES = {.25, 1.8, .9, 1.2, 2.1, .7, 1.4, .6, 1.1, .8};

    private record Character(int site, List<Integer> states) {}
    private record Partition(int states, boolean ordered) implements Comparable<Partition> {
        public int compareTo(Partition other) {
            int bySize = Integer.compare(states, other.states);
            return bySize != 0 ? bySize : Boolean.compare(ordered, other.ordered);
        }
    }

    private static Map<Partition, List<Character>> partitions(Alignment data, Set<Integer> ordered) {
        Map<Partition, List<Character>> groups = new TreeMap<>();
        for (int site = 0; site < data.getSiteCount(); site++) {
            TreeSet<Integer> states = new TreeSet<>();
            for (int code : data.getPattern(data.getPatternIndex(site))) {
                String symbol = data.getDataType().getCode(code);
                if (!symbol.equals("?") && !symbol.equals("-")) {
                    for (int state : data.getDataType().getStatesForCode(code)) states.add(state);
                }
            }
            if (ordered.contains(site) && !states.isEmpty()) {
                int last = states.last();
                for (int state = 0; state <= last; state++) states.add(state);
            }
            if (states.size() < 2) throw new IllegalArgumentException("Unknown state count at " + site);
            groups.computeIfAbsent(new Partition(states.size(), ordered.contains(site)), k -> new ArrayList<>())
                .add(new Character(site, new ArrayList<>(states)));
        }
        return groups;
    }

    private static Alignment alignment(Alignment input, int k, List<Character> sites, boolean variable, boolean informative) throws Exception {
        List<int[]> excluded = variable ? excludedPatterns(k, input.getTaxonCount(), informative) : List.of();
        List<Sequence> sequences = new ArrayList<>();
        TreeSet<String> ambiguities = new TreeSet<>();
        for (int taxon = 0; taxon < input.getTaxonCount(); taxon++) {
            StringBuilder value = new StringBuilder();
            for (Character character : sites) {
                int code = input.getPattern(input.getPatternIndex(character.site()))[taxon];
                String symbol = input.getDataType().getCode(code);
                if (symbol.equals("?") || symbol.equals("-")) {
                    value.append('?');
                    continue;
                }
                StringBuilder allowed = new StringBuilder();
                for (int state : input.getDataType().getStatesForCode(code)) {
                    allowed.append(character.states().indexOf(state));
                }
                if (allowed.length() > 1) {
                    ambiguities.add(allowed.toString());
                    value.append('{').append(allowed).append('}');
                } else value.append(allowed);
            }
            for (int[] pattern : excluded) value.append(pattern[taxon]);
            Sequence sequence = new Sequence();
            sequence.initByName("taxon", input.getTaxaNames().get(taxon), "totalcount", k, "value", value.toString());
            sequences.add(sequence);
        }
        StandardData type = new StandardData();
        type.initByName("nrOfStates", k, "ambiguities", String.join(" ", ambiguities));
        Alignment result = new Alignment();
        result.initByName("sequence", sequences, "userDataType", type, "ascertained", variable,
            "excludefrom", variable ? sites.size() : 0, "excludeto", variable ? sites.size() + excluded.size() : 0);
        return result;
    }

    // This deliberately enumerates excluded columns. It is an independent,
    // small-tree oracle, not the recurrence used by the Rust engine.
    private static List<int[]> excludedPatterns(int k, int tips, boolean informative) {
        List<int[]> patterns = new ArrayList<>();
        if (!informative) {
            for (int state = 0; state < k; state++) {
                int[] pattern = new int[tips];
                java.util.Arrays.fill(pattern, state);
                patterns.add(pattern);
            }
            return patterns;
        }
        if (tips > 8 || k > 4) throw new IllegalArgumentException("Enumeration limited to small fixtures");
        int total = (int) Math.pow(k, tips);
        for (int code = 0; code < total; code++) {
            int[] pattern = new int[tips];
            int[] counts = new int[k];
            int value = code;
            for (int tip = 0; tip < tips; tip++) {
                pattern[tip] = value % k;
                value /= k;
                counts[pattern[tip]]++;
            }
            if (java.util.Arrays.stream(counts).filter(n -> n >= 2).count() < 2) patterns.add(pattern);
        }
        return patterns;
    }

    private static Tree tree(List<String> taxa, double scale) {
        Node[] nodes = new Node[HEIGHTS.length];
        for (int i = 0; i < nodes.length; i++) {
            nodes[i] = new Node();
            nodes[i].setNr(i);
            nodes[i].setHeight(HEIGHTS[i] * scale);
            if (i < 6) nodes[i].setID(taxa.get(i));
            else for (int child : CHILDREN[i - 6]) nodes[i].addChild(nodes[child]);
        }
        return new Tree(nodes[10]);
    }

    public static void main(String[] args) throws Exception {
        boolean informative = args.length > 2 && args[2].equals("informative");
        System.setProperty("java.only", "true");
        PackageManager.loadExternalJars();
        NexusParser parser = new NexusParser();
        parser.parseFile(new File(args[0]));
        Alignment input = parser.m_alignment;
        if (input.getTaxonCount() != 6) throw new IllegalArgumentException("Expected the six-taxon fixture");
        Set<Integer> ordered = new HashSet<>();
        if (args.length > 3 && !args[3].isEmpty()) {
            for (String index : args[3].split(",")) {
                int site = Integer.parseInt(index) - 1;
                if (site < 0 || site >= input.getSiteCount() || !ordered.add(site)) {
                    throw new IllegalArgumentException("Invalid or duplicate ordered character");
                }
            }
        }
        if (informative && !ordered.isEmpty()) throw new IllegalArgumentException("Ordered informative coding not supported by this oracle");
        Map<Partition, List<Character>> groups = partitions(input, ordered);
        JSONArray cases = new JSONArray();
        for (double scale : new double[]{.01, 1, 10}) {
            for (boolean relaxed : new boolean[]{false, true}) {
                for (boolean variable : informative ? new boolean[]{true} : new boolean[]{false, true}) {
                    Tree tree = tree(input.getTaxaNames(), scale);
                    BranchRateModel clock;
                    if (relaxed) {
                        LogNormalDistributionModel distribution = new LogNormalDistributionModel();
                        distribution.initByName("M", new RealParameter("1"), "S", new RealParameter("0.2"), "meanInRealSpace", true);
                        UCRelaxedClockModel model = new UCRelaxedClockModel();
                        Double[] values = java.util.Arrays.stream(RATES).boxed().toArray(Double[]::new);
                        model.initByName("tree", tree, "rates", new RealParameter(values), "distr", distribution);
                        clock = model;
                    } else {
                        StrictClockModel model = new StrictClockModel();
                        model.initAndValidate();
                        clock = model;
                    }
                    JSONArray contributions = new JSONArray();
                    double total = 0;
                    for (var entry : groups.entrySet()) {
                        int k = entry.getKey().states();
                        Alignment data = alignment(input, k, entry.getValue(), variable, informative);
                        SubstitutionModel.Base substitution;
                        if (entry.getKey().ordered()) {
                            Frequencies frequencies = new Frequencies();
                            Double[] uniform = new Double[k];
                            java.util.Arrays.fill(uniform, 1.0 / k);
                            frequencies.initByName("frequencies", new RealParameter(uniform));
                            substitution = new Ordinal();
                            substitution.initByName("stateNumber", k, "frequencies", frequencies);
                        } else {
                            substitution = new LewisMK();
                            substitution.initByName("stateNumber", k);
                        }
                        SiteModel siteModel = new SiteModel();
                        siteModel.initByName("substModel", substitution, "gammaCategoryCount", relaxed ? 4 : 1,
                            "shape", new RealParameter("1"));
                        TreeLikelihood likelihood = new TreeLikelihood();
                        likelihood.initByName("tree", tree, "data", data, "siteModel", siteModel,
                            "branchRateModel", clock, "useAmbiguities", true, "scaling", "always");
                        double logL = likelihood.calculateLogP();
                        if (!Double.isFinite(logL)) throw new IllegalStateException("Nonfinite likelihood");
                        total += logL;
                        contributions.put(new JSONObject().put("states", k).put("ordering", entry.getKey().ordered() ? "ordered" : "unordered").put("log_likelihood", logL)
                            .put("sites", new JSONArray(entry.getValue().stream().map(c -> c.site() + 1).toList())));
                    }
                    cases.put(new JSONObject().put("scale", scale).put("relaxed", relaxed).put("variable", variable)
                        .put("partitions", contributions).put("log_likelihood", total));
                }
            }
        }
        JSONObject result = new JSONObject().put("beast", "2.7.8").put("mm", "1.2.1")
            .put("ordered_characters", new JSONArray(ordered.stream().sorted().map(i -> i + 1).toList()))
            .put("conditioning", informative ? "informative" : "variable-and-none")
            .put("taxa", new JSONArray(input.getTaxaNames())).put("heights", new JSONArray(HEIGHTS)).put("children", new JSONArray(CHILDREN))
            .put("branch_rates", new JSONArray(RATES)).put("gamma_shape", 1).put("clock_sigma", .2).put("cases", cases);
        Files.writeString(Path.of(args[1]), result.toString(2) + "\n");
    }
}
