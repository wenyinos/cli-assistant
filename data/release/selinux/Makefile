SELINUXTYPE = targeted
NULL =

MODULE = cli_assistant.pp.bz2
MODULE_IF = cli_assistant.if

dist_noinst_DATA =	\
	cli_assistant.fc	\
	cli_assistant.if	\
	cli_assistant.te	\
	$(NULL)

selinuxpolicydir = $(datarootdir)/selinux/packages/$(SELINUXTYPE)
nodist_selinuxpolicy_DATA =	\
	$(MODULE)			\
	$(NULL)

selinux_makefile = /usr/share/selinux/devel/Makefile

%.pp.bz2: %.pp
	bzip2 -f -9 $^

%.pp: %.te %.fc %.if
	make -f $(selinux_makefile) $@

clean-local:
	rm -rf *~ *.tc *.pp *.pp.bz2 tmp *.cil

clean: clean-local

test:
	sudo semodule -i cli_assistant.pp

%.cil: %.pp
	cat cli_assistant.pp | /usr/libexec/selinux/hll/pp > cli_assistant.cil
