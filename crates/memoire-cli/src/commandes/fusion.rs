//! Fusion d'une branche dans la branche active (FUSION.md, section 9.9 ; D19, D23) : « fusion plan »
//! montre le plan et son code ; « fusionner » l'applique, après une sauvegarde.

use std::collections::BTreeSet;

use memoire_domain::{
    BranchId, Command, Conflict, MemberField, MergePlan, NodeId, Relation, Resolution, ScopeId,
    Side, Snapshot, Visibility,
};

use super::base::champ_tsv;
use super::depot::sauvegarde_horodatee;
use super::notes::{DECISIONS, IMPLEMENTATIONS, VERDICTS};
use super::reserves::ETATS;
use super::{branche, confirmer, designation, mot, nom_de_branche, Commande, Contexte, Depot};
use crate::arguments::Arguments;
use crate::Echec;

/// Mot de chaque type de relation, pour désigner les relations candidates.
pub(crate) const TYPES: &[(&str, Relation)] = &[
    ("classe-dans", Relation::ClassifiedIn),
    ("depend-de", Relation::DependsOn),
    ("developpe-a-partir-de", Relation::DevelopedFrom),
    ("lien", Relation::Related),
    ("remplace", Relation::Replaces),
    ("restreint", Relation::RestrictsScope),
    ("contredit", Relation::Contradicts),
    ("concerne", Relation::AppliesTo),
    ("traite", Relation::Treats),
];

/// Code du plan : les 8 premiers caractères de la tête de la cible, puis de la source. Il change
/// dès que l'une des deux têtes bouge, donc dès que les numéros pourraient désigner autre chose.
fn code(plan: &MergePlan) -> String {
    let debut = |id: String| id.chars().take(8).collect::<String>();
    format!(
        "{}-{}",
        debut(plan.target_head.to_string()),
        debut(plan.source_head.to_string())
    )
}

/// Valeur d'un champ de membre dans un état, pour l'affichage.
fn valeur_de_champ(etat: &Snapshot, conflit: &Conflict) -> String {
    let Conflict::Field { scope, node, field } = conflit else {
        return String::new();
    };
    valeur_de_membre(etat, *scope, *node, *field)
}

/// Nom d'un champ de membre, tel que la ligne de commande l'écrit.
pub(crate) fn nom_de_champ(field: MemberField) -> &'static str {
    match field {
        MemberField::Verdict => "verdict",
        MemberField::Decision => "decision",
        MemberField::Implementation => "implementation",
        MemberField::Reserve => "reserve",
        MemberField::Visibility => "visibilite",
    }
}

/// Valeur d'un champ d'une note dans un contexte d'un état, pour l'affichage.
pub(crate) fn valeur_de_membre(
    etat: &Snapshot,
    scope: ScopeId,
    node: NodeId,
    field: MemberField,
) -> String {
    let Some(n) = etat.graph(scope).ok().and_then(|g| g.node(node).ok()) else {
        return "-".to_owned();
    };
    let ou = |v: Option<&'static str>| v.unwrap_or("-").to_owned();
    match field {
        MemberField::Verdict => ou(n.verdict().map(|v| mot(v, VERDICTS))),
        MemberField::Decision => ou(n.decision().map(|v| mot(v, DECISIONS))),
        MemberField::Implementation => ou(n.implementation().map(|v| mot(v, IMPLEMENTATIONS))),
        MemberField::Reserve => ou(n.reserve_status().map(|v| mot(v, ETATS))),
        MemberField::Visibility => match n.visibility() {
            Visibility::Current => "courante".to_owned(),
            Visibility::Archived => "archivee".to_owned(),
        },
    }
}

/// Texte courant d'une note dans un état.
fn texte(etat: &Snapshot, conflit: &Conflict) -> String {
    let Conflict::Text { node } = conflit else {
        return String::new();
    };
    etat.revisions(*node)
        .ok()
        .and_then(|r| r.last().map(|r| r.text().to_owned()))
        .unwrap_or_default()
}

/// Ligne d'un conflit, avec la valeur de chaque côté.
fn conflit(cible: &Snapshot, source: &Snapshot, c: &Conflict) -> String {
    match c {
        Conflict::Text { node } => format!(
            "texte de {} — cible : « {} » ; source : « {} »",
            designation(cible, *node),
            texte(cible, c),
            texte(source, c)
        ),
        Conflict::Field { node, field, .. } => {
            let nom = nom_de_champ(*field);
            format!(
                "{nom} de {} — cible : {} ; source : {}",
                designation(cible, *node),
                valeur_de_champ(cible, c),
                valeur_de_champ(source, c)
            )
        }
        Conflict::Ack {
            dependent, cause, ..
        } => format!(
            "acquittement de {} envers {} — différent de chaque côté",
            designation(cible, *dependent),
            designation(cible, *cause)
        ),
    }
}

/// Plan, résolutions comprises, et état de la source.
fn planifier(depot: &Depot, source: BranchId) -> Result<MergePlan, Echec> {
    depot
        .repository()
        .merge_plan(source, &[])
        .map_err(Echec::Refus)
}

fn rien_a_fusionner(source: BranchId, cible: BranchId) -> String {
    format!(
        "Rien à fusionner : {} est déjà réunie dans {}.\n",
        nom_de_branche(source),
        nom_de_branche(cible)
    )
}

/// Titre d'une note, nettoyé pour une ligne --tsv.
fn titre_tsv(etat: &Snapshot, note: NodeId) -> String {
    let titre = etat
        .graph(memoire_domain::DEFAULT_SCOPE)
        .ok()
        .and_then(|g| g.node(note).ok().map(|n| n.title().to_owned()))
        .unwrap_or_else(|| note.to_string());
    champ_tsv(&titre)
}

/// « fusion plan --tsv » (ACTIONS.md, section 7) : code, base, changements sans conflit, un
/// désaccord par ligne, conflit structurel et ses relations candidates, notes à revoir.
fn plan_en_tsv(cible: &Snapshot, source: &Snapshot, plan: &MergePlan) -> String {
    let mut sortie = format!(
        "code\t{}\nbase\t{}\nsans-conflit\t{}\n",
        code(plan),
        plan.base,
        plan.from_source
    );
    for (rang, c) in (1..).zip(&plan.conflicts) {
        let (genre, note, champ, ici, piste) = match c {
            Conflict::Text { node } => (
                "texte",
                *node,
                "texte".to_owned(),
                texte(cible, c),
                texte(source, c),
            ),
            Conflict::Field { node, field, .. } => (
                "champ",
                *node,
                nom_de_champ(*field).to_owned(),
                valeur_de_champ(cible, c),
                valeur_de_champ(source, c),
            ),
            Conflict::Ack {
                dependent, cause, ..
            } => (
                "acquittement",
                *dependent,
                cause.to_string(),
                "-".to_owned(),
                "-".to_owned(),
            ),
        };
        sortie.push_str(&format!(
            "conflit\t{rang}\t{genre}\t{note}\t{}\t{champ}\t{}\t{}\n",
            titre_tsv(cible, note),
            champ_tsv(&ici),
            champ_tsv(&piste)
        ));
    }
    if let Some(structurel) = &plan.structural {
        sortie.push_str(&format!(
            "boucle\t{}\n",
            champ_tsv(&structurel.reason.to_string())
        ));
        for (rang, (scope, edge)) in (1..).zip(&structurel.candidates) {
            let relation = [cible, source]
                .into_iter()
                .find_map(|etat| etat.graph(*scope).ok()?.edge(*edge).ok().cloned());
            if let Some(r) = relation {
                sortie.push_str(&format!(
                    "candidate\t{rang}\t{edge}\t{}\t{}\t{}\n",
                    titre_tsv(cible, r.from()),
                    mot(r.relation(), TYPES),
                    titre_tsv(cible, r.to())
                ));
            }
        }
    }
    for ((_, note), changement) in &plan.effects.review {
        let quoi = if changement.after.is_empty() {
            "plus-a-revoir"
        } else {
            "a-revoir"
        };
        sortie.push_str(&format!("{quoi}\t{note}\t{}\n", titre_tsv(cible, *note)));
    }
    sortie
}

pub(crate) struct FusionPlan;

impl Commande for FusionPlan {
    fn nom(&self) -> &'static str {
        "fusion plan"
    }
    fn usage(&self) -> &'static str {
        "SOURCE [--tsv]"
    }
    fn resume(&self) -> &'static str {
        "plan de la fusion de SOURCE dans la branche active : base, conflits numérotés, conflit \
         structurel, effets, code du plan ; --tsv : une donnée par ligne, pour l'interface"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let en_tsv = arguments.drapeau("--tsv")?;
        let saisie = arguments.positionnel("SOURCE")?;
        arguments.fin()?;
        let depot = contexte.ouvrir()?;
        let source = branche(&depot, &saisie)?;
        let plan = planifier(&depot, source)?;
        let repo = depot.repository();
        let active = repo.active_branch();
        if plan.up_to_date {
            if en_tsv {
                return contexte.ecrire("a-jour\toui\n");
            }
            return contexte.ecrire(&rien_a_fusionner(source, active));
        }
        let cible = repo.snapshot();
        let etat_source = repo.snapshot_at(plan.source_head).map_err(Echec::Refus)?;
        if en_tsv {
            return contexte.ecrire(&plan_en_tsv(cible, &etat_source, &plan));
        }
        let mut texte = format!(
            "Fusion de {} dans {} :\nBase : {}\nRéunis sans conflit : {}\nConflits :\n",
            nom_de_branche(source),
            nom_de_branche(active),
            plan.base,
            plan.from_source
        );
        if plan.conflicts.is_empty() {
            texte.push_str("  (aucun)\n");
        }
        for (rang, c) in (1..).zip(&plan.conflicts) {
            texte.push_str(&format!("  {rang}. {}\n", conflit(cible, &etat_source, c)));
        }
        if let Some(structurel) = &plan.structural {
            texte.push_str(&format!(
                "Conflit structurel : {}\nRelations candidates :\n",
                structurel.reason
            ));
            for (rang, (scope, edge)) in (1..).zip(&structurel.candidates) {
                let relation = [cible, &*etat_source]
                    .into_iter()
                    .find_map(|etat| etat.graph(*scope).ok()?.edge(*edge).ok().cloned());
                let detail = relation.map_or_else(String::new, |r| {
                    format!(
                        " : {} {} {}",
                        designation(cible, r.from()),
                        mot(r.relation(), TYPES),
                        designation(cible, r.to())
                    )
                });
                texte.push_str(&format!("  {rang}. {edge}{detail}\n"));
            }
        }
        texte.push_str("Réexamens :\n");
        if plan.effects.review.is_empty() {
            texte.push_str("  (aucun changement)\n");
        }
        for ((_, note), changement) in &plan.effects.review {
            let quoi = if changement.after.is_empty() {
                "n'est plus à réexaminer"
            } else {
                "à réexaminer"
            };
            texte.push_str(&format!("  - {quoi} : {}\n", designation(cible, *note)));
        }
        texte.push_str(&format!("Code du plan : {}\n", code(&plan)));
        contexte.ecrire(&texte)
    }
}

/// Numéros saisis (« 1,3 »), chacun dans 1..=borne et jamais déjà vu.
fn numeros(
    liste: Option<String>,
    borne: usize,
    vus: &mut BTreeSet<usize>,
    option: &str,
) -> Result<Vec<usize>, Echec> {
    let Some(liste) = liste else {
        return Ok(Vec::new());
    };
    let mut trouves = Vec::new();
    for saisie in liste.split(',').map(str::trim) {
        let rang: usize = saisie
            .parse()
            .map_err(|_| Echec::Usage(format!("{option} : numéro invalide « {saisie} »")))?;
        if rang == 0 || rang > borne {
            return Err(Echec::Usage(format!(
                "{option} : aucun élément numéro {rang} dans le plan"
            )));
        }
        if !vus.insert(rang) {
            return Err(Echec::Usage(format!("numéro {rang} cité deux fois")));
        }
        trouves.push(rang);
    }
    Ok(trouves)
}

pub(crate) struct Fusionner;

impl Commande for Fusionner {
    fn nom(&self) -> &'static str {
        "fusionner"
    }
    fn usage(&self) -> &'static str {
        "SOURCE --plan CODE [--cible NUMEROS] [--source NUMEROS] [--desactiver NUMEROS] \
         [--motif M] [--oui]"
    }
    fn resume(&self) -> &'static str {
        "fusionne SOURCE selon son plan (NUMEROS : numéros séparés par des virgules), après une \
         sauvegarde dans sauvegardes/"
    }
    fn executer(&self, arguments: Arguments, contexte: &mut Contexte<'_>) -> Result<(), Echec> {
        let mut arguments = arguments;
        let attendu = arguments.requise("--plan")?;
        let vers_cible = arguments.option("--cible")?;
        let vers_source = arguments.option("--source")?;
        let desactiver = arguments.option("--desactiver")?;
        let motif = arguments.option("--motif")?.unwrap_or_default();
        let confirme = arguments.drapeau("--oui")?;
        let saisie = arguments.positionnel("SOURCE")?;
        arguments.fin()?;
        let mut depot = contexte.ouvrir()?;
        let source = branche(&depot, &saisie)?;
        // Le plan est recalculé : son code garantit que les numéros désignent les conflits lus.
        let plan = planifier(&depot, source)?;
        if code(&plan) != attendu {
            return Err(Echec::Usage(format!(
                "plan périmé : relancer « memoire fusion plan {saisie} »"
            )));
        }
        let mut vus = BTreeSet::new();
        let mut resolutions = Vec::new();
        for (liste, cote, option) in [
            (vers_cible, Side::Target, "--cible"),
            (vers_source, Side::Source, "--source"),
        ] {
            for rang in numeros(liste, plan.conflicts.len(), &mut vus, option)? {
                resolutions.push(Resolution::Choose(plan.conflicts[rang - 1], cote));
            }
        }
        let candidates = plan.structural.map(|s| s.candidates).unwrap_or_default();
        for rang in numeros(
            desactiver,
            candidates.len(),
            &mut BTreeSet::new(),
            "--desactiver",
        )? {
            let (scope, edge) = candidates[rang - 1];
            resolutions.push(Resolution::Deactivate { scope, edge });
        }
        let active = depot.repository().active_branch();
        let source_head = plan.source_head;
        let prevu = depot.plan(|_| Command::Merge {
            source,
            source_head,
            resolutions,
        })?;
        if !prevu.preview.publishes {
            return contexte.ecrire(&rien_a_fusionner(source, active));
        }
        confirmer(contexte, &depot, &prevu.preview, confirme)?;
        // Filet de sécurité (D19) : sans sauvegarde écrite, rien n'est fusionné.
        let copie = sauvegarde_horodatee(&contexte.base, "avant-fusion");
        if let Some(dossier) = copie.parent() {
            std::fs::create_dir_all(dossier)?;
        }
        depot.backup(&copie)?;
        let applique = depot.apply(prevu, &motif)?;
        contexte.ecrire(&format!(
            "Sauvegarde écrite : {}\nFusion enregistrée : {}\n",
            copie.display(),
            applique.outcome.head
        ))
    }
}
