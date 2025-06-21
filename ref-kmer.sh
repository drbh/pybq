# jellyfish count -m 4 -s 100M -t 16 GCF_000001635.27_GRCm39_genomic.fna
jellyfish count -m 4 -s 100M -t 16 mouse.fa
jellyfish dump mer_counts.jf > mer_counts_dumps.fa
head -n 10 mer_counts_dumps.fa