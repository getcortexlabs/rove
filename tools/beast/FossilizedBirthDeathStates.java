import beast.base.evolution.tree.Node;
import beast.base.evolution.tree.Tree;
import beast.base.inference.parameter.RealParameter;
import sa.evolution.speciation.SABirthDeathModel;

/** Fixed states for the BEAST 2.7.8 / SA 2.1.1 FBD density comparison. */
public class FossilizedBirthDeathStates {
    static RealParameter parameter(double x) { return new RealParameter(Double.toString(x)); }
    public static void main(String[] args) {
        double[][] cases = {
            {0.8,0.3,0.2,1,5,0,0.7,1.3,2.1},
            {0.8,0.3,0.2,0.4,5,0,0.7,1.3,2.1},
            {0.2,0.8,0.4,0.7,8,0,1.1,2.0,4.2},
            {0.5,0.5,0.15,0.8,6,0,0.2,1.5,3.0},
            {0.8,0.3,0.2,0,5,0.2,0.7,1.3,2.1},
            {0.8,0.3,0.2,1,5,0,1e-9,1.3,2.1},
            {0.08,0.03,0.02,0.5,50,0,7,13,21},
            {0.8,0,0.2,1,5,0,0,1.3,2.1}
        };
        System.out.println("birth\tdeath\tfossil_sampling\tpresent_sampling\torigin\tage_a\tage_b\tinternal\troot\tlog_density");
        for (double[] c : cases) {
            Node[] n = new Node[5];
            double[] h = {c[5], c[6], c[5], c[7], c[8]};
            for (int i=0; i<5; i++) {
                n[i] = new Node(); n[i].setNr(i); n[i].setHeight(h[i]);
                if (i<3) n[i].setID("T"+(i+1));
            }
            n[3].addChild(n[0]); n[3].addChild(n[1]);
            n[4].addChild(n[3]); n[4].addChild(n[2]);
            Tree tree = new Tree(n[4]);
            SABirthDeathModel prior = new SABirthDeathModel();
            prior.initByName("tree", tree, "birthRate", parameter(c[0]),
                "deathRate", parameter(c[1]), "samplingRate", parameter(c[2]),
                "rho", parameter(c[3]), "origin", parameter(c[4]),
                "removalProbability", parameter(0), "conditionOnSampling", false,
                "conditionOnRhoSampling", false, "conditionOnRoot", false);
            for (double x : c) System.out.print(x+"\t");
            System.out.println(prior.calculateLogP());
        }
    }
}
