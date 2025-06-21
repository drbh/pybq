download-mouse:
	@echo "Downloading mouse data..."
	@curl -OL https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/001/635/GCF_000001635.27_GRCm39/GCF_000001635.27_GRCm39_genomic.fna.gz

encode-mouse:
	@echo "Converting mouse genome to VBQ format..."
	@bqtools encode -B 62239136 -f a GCF_000001635.27_GRCm39_genomic.fna.gz -o mouse.vbq
