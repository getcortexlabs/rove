import beast.base.evolution.alignment.Alignment;
import beast.base.evolution.alignment.Sequence;
import beast.base.evolution.branchratemodel.StrictClockModel;
import beast.base.evolution.datatype.StandardData;
import beast.base.evolution.likelihood.TreeLikelihood;
import beast.base.evolution.sitemodel.SiteModel;
import beast.base.evolution.substitutionmodel.JukesCantor;
import beast.base.evolution.substitutionmodel.SubstitutionModel;
import beast.base.evolution.tree.Node;
import beast.base.evolution.tree.Tree;
import beast.base.inference.parameter.RealParameter;
import morphmodels.evolution.substitutionmodel.LewisMK;
import java.io.PrintStream;
import java.util.ArrayList;

/** Fixed shared-tree states for Mk/ascertained morphology + JC69 DNA. */
public class MixedStates {
    static final String[] NAMES = {"Alpha_one","Beta_two","Gamma_three","Delta_four","Epsilon_five"};
    static final String[] MORPH = {"010101","101010","011001","100110","110011"};
    static final String[] DNA = {"AACGTACGTAAC","AATGTCATACGT","ACCGTGGTAAGT","AGCGTTACTCGT","ATCGTCGTACGT"};
    static Alignment data(boolean dna, int conditioning) throws Exception {
        var excluded = new ArrayList<Integer>();
        for (int bits=0;bits<32;++bits) {
            int ones = Integer.bitCount(bits);
            if (conditioning == 1 && (ones==0 || ones==5) || conditioning==2 && (ones<2 || ones>3)) excluded.add(bits);
        }
        var sequences = new ArrayList<Sequence>();
        for (int t=0;t<5;++t) {
            var row = new StringBuilder(dna ? DNA[t] : MORPH[t]);
            if (!dna) for (int bits:excluded) row.append((bits>>t)&1);
            Sequence sequence = new Sequence();
            sequence.initByName("taxon",NAMES[t],"value",row.toString(),"totalcount",dna?4:2);
            sequences.add(sequence);
        }
        Alignment alignment = new Alignment();
        if (dna) alignment.initByName("sequence",sequences,"dataType","nucleotide");
        else {
            StandardData type = new StandardData(); type.initByName("nrOfStates",2);
            alignment.initByName("sequence",sequences,"userDataType",type,"ascertained",conditioning>0,
                "excludefrom",conditioning>0?6:0,"excludeto",conditioning>0?6+excluded.size():0);
        }
        return alignment;
    }
    static double likelihood(Alignment data, Tree tree, boolean dna, int categories, double ratio) throws Exception {
        SubstitutionModel model;
        if (dna) { JukesCantor jc=new JukesCantor();jc.initAndValidate();model=jc; }
        else { LewisMK mk=new LewisMK();mk.initByName("stateNumber",2);model=mk; }
        SiteModel site=new SiteModel();site.initByName("substModel",model,"gammaCategoryCount",categories,"shape",new RealParameter("0.7"));
        StrictClockModel clock=new StrictClockModel();clock.initByName("clock.rate",new RealParameter(Double.toString(ratio)));
        TreeLikelihood likelihood=new TreeLikelihood();likelihood.initByName("tree",tree,"data",data,"siteModel",site,"branchRateModel",clock,"useAmbiguities",true,"scaling","always");
        return likelihood.calculateLogP();
    }
    public static void main(String[] args) throws Exception {
        System.setProperty("java.only","true");beast.pkgmgmt.PackageManager.loadExternalJars();
        try(PrintStream out=new PrintStream(args[0])) {
            out.println("height_step\tcategories\tconditioning\tmorphology_rate\tmorphology_log_likelihood\tdna_log_likelihood\tjoint_log_likelihood");
            for(double step:new double[]{0.01,0.2}) for(int categories:new int[]{1,4}) for(int conditioning=0;conditioning<3;++conditioning) for(double ratio:new double[]{0.2,1.0,3.5}) {
                Node[] nodes=new Node[9];
                for(int i=0;i<9;++i) {nodes[i]=new Node();nodes[i].setNr(i);nodes[i].setHeight(i<5?0:(i-4)*step);if(i<5)nodes[i].setID(NAMES[i]);}
                nodes[5].addChild(nodes[0]);nodes[5].addChild(nodes[1]);
                for(int i=6;i<9;++i){nodes[i].addChild(nodes[i-1]);nodes[i].addChild(nodes[i-4]);}
                Tree tree=new Tree(nodes[8]);
                double morph=likelihood(data(false,conditioning),tree,false,categories,ratio);
                double dna=likelihood(data(true,0),tree,true,categories,1.0);
                out.println(step+"\t"+categories+"\t"+conditioning+"\t"+ratio+"\t"+morph+"\t"+dna+"\t"+(morph+dna));
            }
        }
    }
}
