import beast.base.evolution.speciation.YuleModel;
import beast.base.evolution.speciation.BirthDeathGernhard08Model;
import beast.base.evolution.tree.Node;
import beast.base.evolution.tree.Tree;
import beast.base.inference.parameter.RealParameter;

/** Fixed, synthetic states for BEAST.base 2.7.8. No MCMC or timing study. */
public class YuleStates {
    static RealParameter parameter(double x) { return new RealParameter(Double.toString(x)); }
    public static void main(String[] args) {
        System.out.println("tips\tbirth_rate\theight_step\tyule_log_density\tzero_death_log_density\tlabeled_log_density");
        for (int tips : new int[]{2,3,5,8}) {
            for (double rate : new double[]{0.02,1.3,200.0}) {
                for (double step : new double[]{0.001,0.7}) {
                    Node[] nodes = new Node[2*tips-1];
                    for (int i=0; i<nodes.length; ++i) {
                        nodes[i] = new Node(); nodes[i].setNr(i);
                        nodes[i].setHeight(i<tips ? 0.0 : (i-tips+1)*step);
                        if (i<tips) nodes[i].setID("T"+i);
                    }
                    nodes[tips].addChild(nodes[0]); nodes[tips].addChild(nodes[1]);
                    for (int i=tips+1; i<nodes.length; ++i) {
                        nodes[i].addChild(nodes[i-1]); nodes[i].addChild(nodes[i-tips+1]);
                    }
                    Tree tree = new Tree(nodes[nodes.length-1]);
                    YuleModel yule = new YuleModel();
                    yule.initByName("tree",tree,"birthDiffRate",parameter(rate));
                    BirthDeathGernhard08Model bd = new BirthDeathGernhard08Model();
                    bd.initByName("tree",tree,"birthDiffRate",parameter(rate),"relativeDeathRate",parameter(0),"type","unscaled");
                    BirthDeathGernhard08Model labeled = new BirthDeathGernhard08Model();
                    labeled.initByName("tree",tree,"birthDiffRate",parameter(rate),"relativeDeathRate",parameter(0),"type","labeled");
                    System.out.println(tips+"\t"+rate+"\t"+step+"\t"+yule.calculateLogP()+"\t"+bd.calculateLogP()+"\t"+labeled.calculateLogP());
                }
            }
        }
    }
}
