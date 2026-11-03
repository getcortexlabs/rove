import beast.base.evolution.alignment.Alignment;
import beast.base.evolution.alignment.Sequence;
import beast.base.evolution.branchratemodel.StrictClockModel;
import beast.base.evolution.likelihood.TreeLikelihood;
import beast.base.evolution.sitemodel.SiteModel;
import beast.base.evolution.substitutionmodel.JukesCantor;
import beast.base.evolution.tree.Node;
import beast.base.evolution.tree.Tree;
import beast.base.inference.parameter.RealParameter;
import java.io.PrintStream;
import java.util.ArrayList;

/** Synthetic DNA likelihood states for pinned BEAST.base 2.7.8; no MCMC. */
public class DnaStates {
    public static void main(String[] args) throws Exception {
        System.setProperty("java.only", "true");
        beast.pkgmgmt.PackageManager.loadExternalJars();
        String[] names = {"Alpha_one", "Beta_two", "Gamma_three", "Delta_four", "Epsilon_five"};
        String[] rows = {"AACGTRYN?-AC", "AATGTCATACGT", "ACCGTGGTAAGT", "AGCGTTACTCGT", "ATCGTCGTACGT"};
        var sequences = new ArrayList<Sequence>();
        for (int i=0; i<names.length; ++i) {
            Sequence sequence = new Sequence();
            sequence.initByName("taxon", names[i], "value", rows[i], "totalcount", 4);
            sequences.add(sequence);
        }
        Alignment data = new Alignment();
        data.initByName("sequence", sequences, "dataType", "nucleotide");
        try (PrintStream out = new PrintStream(args[0])) {
            out.println("height_step\tcategories\tshape\tlog_likelihood\tcategory_rates");
            for (double step : new double[]{0.001,0.05,1.7}) {
                for (int categories : new int[]{1,4}) {
                    Node[] nodes = new Node[9];
                    for (int i=0; i<9; ++i) {
                        nodes[i] = new Node(); nodes[i].setNr(i);
                        nodes[i].setHeight(i<5 ? 0.0 : (i-4)*step);
                        if (i<5) nodes[i].setID(names[i]);
                    }
                    nodes[5].addChild(nodes[0]); nodes[5].addChild(nodes[1]);
                    for (int i=6; i<9; ++i) {
                        nodes[i].addChild(nodes[i-1]); nodes[i].addChild(nodes[i-4]);
                    }
                    Tree tree = new Tree(nodes[8]);
                    JukesCantor jc = new JukesCantor(); jc.initAndValidate();
                    SiteModel site = new SiteModel();
                    site.initByName("substModel", jc, "gammaCategoryCount", categories,
                        "shape", new RealParameter("0.7"));
                    StrictClockModel clock = new StrictClockModel();
                    clock.initByName("clock.rate", new RealParameter("1"));
                    TreeLikelihood likelihood = new TreeLikelihood();
                    likelihood.initByName("tree", tree, "data", data, "siteModel", site,
                        "branchRateModel", clock, "useAmbiguities", true, "scaling", "always");
                    out.println(step+"\t"+categories+"\t0.7\t"+likelihood.calculateLogP()+"\t"+
                        java.util.Arrays.stream(site.getCategoryRates(tree.getRoot()))
                            .mapToObj(Double::toString).collect(java.util.stream.Collectors.joining(",")));
                }
            }
        }
    }
}
