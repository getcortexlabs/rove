import beast.base.evolution.tree.Node;
import beast.base.evolution.tree.Tree;
import beast.base.inference.parameter.RealParameter;
import beast.base.inference.distribution.Beta;
import beast.base.inference.distribution.Exponential;
import sa.evolution.speciation.SABirthDeathModel;

/** Joint sampling density and proper hyperpriors: BEAST 2.7.8 / SA 2.1.1. */
public class FbdEstimationStates {
    static RealParameter parameter(double x) { return new RealParameter(Double.toString(x)); }
    static double exponential(double x, double mean, double offset) {
        Exponential d = new Exponential();
        d.initByName("mean", parameter(mean), "offset", offset);
        return d.logDensity(x);
    }
    public static void main(String[] args) {
        double[][] cases = {
            {0.8,0.3,0.2,0.7,4,0.7,2},
            {0.4,0.3,0.2,0.7,4,0.7,2},
            {0.8,0.8,0.2,0.7,4,0.7,2},
            {0.8,0.3,0.5,0.7,4,0.7,2},
            {0.8,0.3,0.2,0.3,4,0.7,2},
            {0.8,0.3,0.2,0.7,2.1,0.7,2},
            {0.1,0.4,0.6,0.99,10,0.7,7},
            {1.5,0.1,0.05,0.01,4,0.7,1}
        };
        System.out.println("birth\tdeath\tfossil_sampling\tpresent_sampling\torigin\tage\troot\ttree_density\tparameter_prior\tjoint_density");
        for (double[] c: cases) {
            Node a = new Node(); a.setNr(0); a.setID("A"); a.setHeight(0);
            Node b = new Node(); b.setNr(1); b.setID("B"); b.setHeight(c[5]);
            Node r = new Node(); r.setNr(2); r.setHeight(c[6]); r.addChild(a); r.addChild(b);
            SABirthDeathModel f = new SABirthDeathModel();
            f.initByName("tree",new Tree(r),"birthRate",parameter(c[0]),"deathRate",parameter(c[1]),
                "samplingRate",parameter(c[2]),"rho",parameter(c[3]),"origin",parameter(c[4]),
                "removalProbability",parameter(0),"conditionOnSampling",false,
                "conditionOnRhoSampling",false,"conditionOnRoot",false);
            Beta beta = new Beta(); beta.initByName("alpha",parameter(2),"beta",parameter(3));
            double hyper = exponential(c[0],1.1,0)+exponential(c[1],0.6,0)
                +exponential(c[2],0.3,0)+beta.logDensity(c[3])+exponential(c[4],2,c[5]);
            double tree = f.calculateLogP();
            for (double x: c) System.out.print(x+"\t");
            System.out.println(tree+"\t"+hyper+"\t"+(tree+hyper));
        }
    }
}
