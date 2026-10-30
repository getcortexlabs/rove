package rove.reference;

import beast.base.core.BEASTInterface;
import beast.base.evolution.likelihood.GenericTreeLikelihood;
import beast.base.evolution.sitemodel.SiteModel;
import beast.base.evolution.tree.Node;
import beast.base.evolution.tree.Tree;
import beast.base.inference.Distribution;
import beast.base.inference.MCMC;
import beast.base.inference.State;
import beast.base.inference.parameter.RealParameter;
import beast.base.parser.XMLParser;
import beast.pkgmgmt.PackageManager;
import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.IdentityHashMap;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;

/** Multi-partition fixed-state oracle for the expanded benchmark. */
public final class BenchmarkSnapshot {
    private static void visit(BEASTInterface object, Set<BEASTInterface> seen,
                              Map<String, BEASTInterface> objects) throws Exception {
        if (!seen.add(object)) return;
        if (object.getID() != null) objects.put(object.getID(), object);
        for (BEASTInterface child : object.listActiveBEASTObjects()) visit(child, seen, objects);
    }

    public static void main(String[] args) throws Exception {
        PackageManager.loadExternalJars();
        MCMC mcmc = (MCMC) new XMLParser().parseFile(new File(args[0]));
        State state = mcmc.startStateInput.get();
        state.setStateFileName(args[1]);
        state.restoreFromFile();
        double posterior = state.robustlyCalcPosterior(mcmc.posteriorInput.get());
        if (!Double.isFinite(posterior)) throw new IllegalStateException("Nonfinite posterior");
        Map<String, BEASTInterface> objects = new TreeMap<>();
        visit(mcmc, Collections.newSetFromMap(new IdentityHashMap<>()), objects);
        var likelihoods = new ArrayList<GenericTreeLikelihood>();
        for (BEASTInterface object : objects.values()) {
            if (object instanceof GenericTreeLikelihood likelihood) likelihoods.add(likelihood);
        }
        likelihoods.sort(Comparator.comparingInt(l -> l.dataInput.get().getMaxStateCount()));
        var first = likelihoods.get(0);
        Tree tree = (Tree) first.treeInput.get();
        SiteModel site = (SiteModel) first.siteModelInput.get();
        JSONArray nodes = new JSONArray();
        for (Node node : tree.getNodesAsArray()) {
            JSONArray children = new JSONArray();
            for (Node child : node.getChildren()) children.put(child.getNr());
            JSONObject entry = new JSONObject().put("index", node.getNr())
                .put("children", children).put("height", node.getHeight());
            if (node.isLeaf()) entry.put("taxon", node.getID());
            if (!node.isRoot()) entry.put("branch_rate", first.branchRateModelInput.get().getRateForBranch(node));
            nodes.put(entry);
        }
        JSONArray partitions = new JSONArray();
        for (var likelihood : likelihoods) {
            partitions.put(new JSONObject().put("states", likelihood.dataInput.get().getMaxStateCount())
                .put("patterns", likelihood.dataInput.get().getPatternCount())
                .put("implementation", likelihood.getClass().getName())
                .put("log_likelihood", likelihood.getCurrentLogP()));
        }
        JSONObject components = new JSONObject();
        String[][] distributions = {
            {"posterior", "posterior"}, {"likelihood", "likelihood"}, {"prior", "prior"},
            {"birth_death", "BirthDeath.t:matriz_20x50"},
            {"gamma_prior", "GammaShapePrior.s:matriz_20x502"},
            {"clock_rate_prior", "ORCRatePriorDistribution.c:matriz_20x50"},
            {"clock_sigma_prior", "ORCsigmaPrior.c:matriz_20x50"}
        };
        for (String[] pair : distributions) {
            components.put(pair[0], ((Distribution) objects.get(pair[1])).getCurrentLogP());
        }
        JSONObject result = new JSONObject().put("schema_version", 1)
            .put("nodes", nodes).put("root", tree.getRoot().getNr()).put("partitions", partitions)
            .put("components", components).put("category_rates", new JSONArray(site.getCategoryRates(tree.getRoot())))
            .put("category_weights", new JSONArray(site.getCategoryProportions(tree.getRoot())));
        String[][] parameters = {
            {"birth_difference", "BDBirthRate.t:matriz_20x50"},
            {"relative_death", "BDDeathRate.t:matriz_20x50"},
            {"gamma_shape", "gammaShape.s:matriz_20x502"}, {"clock_sigma", "ORCsigma.c:matriz_20x50"}
        };
        for (String[] pair : parameters) result.put(pair[0], ((RealParameter) objects.get(pair[1])).getValue());
        Files.writeString(Path.of(args[2]), result.toString(2) + "\n");
    }
}
