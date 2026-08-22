//! Static HL7 v2 dictionary: segment field definitions, code tables and the
//! abstract message structures used for structural validation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    /// Must be present and populated.
    Required,
    /// Populate when known: missing values are reported as warnings.
    Recommended,
    Optional,
    /// Retained for backward compatibility; senders should stop using it.
    Backward,
}

#[derive(Debug, Clone, Copy)]
pub struct FieldSpec {
    pub seq: usize,
    pub name: &'static str,
    /// HL7 data type, e.g. `CX`, `XPN`, `DTM`.
    pub dt: &'static str,
    pub usage: Use,
    pub table: Option<&'static str>,
    pub repeats: bool,
}

macro_rules! fld {
    ($seq:expr, $name:expr, $dt:expr, $usage:expr) => {
        FieldSpec {
            seq: $seq,
            name: $name,
            dt: $dt,
            usage: $usage,
            table: None,
            repeats: false,
        }
    };
    ($seq:expr, $name:expr, $dt:expr, $usage:expr, rep) => {
        FieldSpec {
            seq: $seq,
            name: $name,
            dt: $dt,
            usage: $usage,
            table: None,
            repeats: true,
        }
    };
    ($seq:expr, $name:expr, $dt:expr, $usage:expr, t $tbl:expr) => {
        FieldSpec {
            seq: $seq,
            name: $name,
            dt: $dt,
            usage: $usage,
            table: Some($tbl),
            repeats: false,
        }
    };
    ($seq:expr, $name:expr, $dt:expr, $usage:expr, t $tbl:expr, rep) => {
        FieldSpec {
            seq: $seq,
            name: $name,
            dt: $dt,
            usage: $usage,
            table: Some($tbl),
            repeats: true,
        }
    };
}

use Use::{Backward as B, Optional as O, Recommended as RE, Required as R};

#[derive(Debug, Clone, Copy)]
pub struct SegmentSpec {
    pub name: &'static str,
    pub desc: &'static str,
    pub fields: &'static [FieldSpec],
}

const MSH: &[FieldSpec] = &[
    fld!(1, "Field Separator", "ST", R),
    fld!(2, "Encoding Characters", "ST", R),
    fld!(3, "Sending Application", "HD", RE),
    fld!(4, "Sending Facility", "HD", RE),
    fld!(5, "Receiving Application", "HD", RE),
    fld!(6, "Receiving Facility", "HD", RE),
    fld!(7, "Date/Time of Message", "DTM", R),
    fld!(8, "Security", "ST", O),
    fld!(9, "Message Type", "MSG", R),
    fld!(10, "Message Control ID", "ST", R),
    fld!(11, "Processing ID", "PT", R, t "0103"),
    fld!(12, "Version ID", "VID", R, t "0104"),
    fld!(13, "Sequence Number", "NM", O),
    fld!(14, "Continuation Pointer", "ST", O),
    fld!(15, "Accept Ack Type", "ID", O, t "0155"),
    fld!(16, "Application Ack Type", "ID", O, t "0155"),
    fld!(17, "Country Code", "ID", O),
    fld!(18, "Character Set", "ID", O, rep),
    fld!(19, "Principal Language", "CE", O),
    fld!(20, "Alt Character Set Handling", "ID", O),
    fld!(21, "Message Profile Identifier", "EI", O, rep),
];

const MSA: &[FieldSpec] = &[
    fld!(1, "Acknowledgment Code", "ID", R, t "0008"),
    fld!(2, "Message Control ID", "ST", R),
    fld!(3, "Text Message", "ST", B),
    fld!(4, "Expected Sequence Number", "NM", O),
    fld!(5, "Delayed Ack Type", "ID", B),
    fld!(6, "Error Condition", "CE", B),
];

const ERR: &[FieldSpec] = &[
    fld!(1, "Error Code and Location", "ELD", B, rep),
    fld!(2, "Error Location", "ERL", O, rep),
    fld!(3, "HL7 Error Code", "CWE", R, t "0357"),
    fld!(4, "Severity", "ID", R, t "0516"),
    fld!(5, "Application Error Code", "CWE", O),
    fld!(7, "Diagnostic Information", "TX", O),
    fld!(8, "User Message", "TX", O),
];

const EVN: &[FieldSpec] = &[
    fld!(1, "Event Type Code", "ID", B, t "0003"),
    fld!(2, "Recorded Date/Time", "DTM", R),
    fld!(3, "Date/Time Planned Event", "DTM", O),
    fld!(4, "Event Reason Code", "IS", O, t "0062"),
    fld!(5, "Operator ID", "XCN", O, rep),
    fld!(6, "Event Occurred", "DTM", RE),
    fld!(7, "Event Facility", "HD", O),
];

const PID: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", O),
    fld!(2, "Patient ID", "CX", B),
    fld!(3, "Patient Identifier List", "CX", R, rep),
    fld!(4, "Alternate Patient ID", "CX", B, rep),
    fld!(5, "Patient Name", "XPN", R, rep),
    fld!(6, "Mother's Maiden Name", "XPN", O, rep),
    fld!(7, "Date/Time of Birth", "DTM", RE),
    fld!(8, "Administrative Sex", "IS", RE, t "0001"),
    fld!(9, "Patient Alias", "XPN", B, rep),
    fld!(10, "Race", "CE", O, t "0005", rep),
    fld!(11, "Patient Address", "XAD", RE, rep),
    fld!(12, "County Code", "IS", B),
    fld!(13, "Home Phone Number", "XTN", O, rep),
    fld!(14, "Business Phone Number", "XTN", O, rep),
    fld!(15, "Primary Language", "CE", O),
    fld!(16, "Marital Status", "CE", O, t "0002"),
    fld!(17, "Religion", "CE", O, t "0006"),
    fld!(18, "Patient Account Number", "CX", O),
    fld!(19, "SSN Number", "ST", B),
    fld!(20, "Driver's License Number", "DLN", B),
    fld!(21, "Mother's Identifier", "CX", O, rep),
    fld!(22, "Ethnic Group", "CE", O, t "0189", rep),
    fld!(23, "Birth Place", "ST", O),
    fld!(24, "Multiple Birth Indicator", "ID", O, t "0136"),
    fld!(25, "Birth Order", "NM", O),
    fld!(26, "Citizenship", "CE", O, rep),
    fld!(27, "Veterans Military Status", "CE", O),
    fld!(28, "Nationality", "CE", B),
    fld!(29, "Patient Death Date/Time", "DTM", O),
    fld!(30, "Patient Death Indicator", "ID", O, t "0136"),
    fld!(31, "Identity Unknown Indicator", "ID", O, t "0136"),
    fld!(32, "Identity Reliability Code", "IS", O, rep),
    fld!(33, "Last Update Date/Time", "DTM", O),
    fld!(34, "Last Update Facility", "HD", O),
    fld!(35, "Species Code", "CE", O),
    fld!(36, "Breed Code", "CE", O),
    fld!(37, "Strain", "ST", O),
    fld!(38, "Production Class Code", "CE", O),
    fld!(39, "Tribal Citizenship", "CWE", O, rep),
];

const PD1: &[FieldSpec] = &[
    fld!(1, "Living Dependency", "IS", O, rep),
    fld!(2, "Living Arrangement", "IS", O),
    fld!(3, "Patient Primary Facility", "XON", O, rep),
    fld!(4, "Patient Primary Care Provider", "XCN", B, rep),
    fld!(6, "Organ Donor Code", "IS", O),
    fld!(11, "Publicity Code", "CE", O),
    fld!(12, "Protection Indicator", "ID", O, t "0136"),
];

const NK1: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Name", "XPN", RE, rep),
    fld!(3, "Relationship", "CE", RE, t "0063"),
    fld!(4, "Address", "XAD", O, rep),
    fld!(5, "Phone Number", "XTN", O, rep),
    fld!(6, "Business Phone Number", "XTN", O, rep),
    fld!(7, "Contact Role", "CE", O, t "0131"),
    fld!(8, "Start Date", "DT", O),
    fld!(9, "End Date", "DT", O),
    fld!(13, "Organization Name", "XON", O, rep),
];

const PV1: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", O),
    fld!(2, "Patient Class", "IS", R, t "0004"),
    fld!(3, "Assigned Patient Location", "PL", RE),
    fld!(4, "Admission Type", "IS", O, t "0007"),
    fld!(5, "Preadmit Number", "CX", O),
    fld!(6, "Prior Patient Location", "PL", O),
    fld!(7, "Attending Doctor", "XCN", RE, rep),
    fld!(8, "Referring Doctor", "XCN", O, rep),
    fld!(9, "Consulting Doctor", "XCN", B, rep),
    fld!(10, "Hospital Service", "IS", O, t "0069"),
    fld!(11, "Temporary Location", "PL", O),
    fld!(12, "Preadmit Test Indicator", "IS", O),
    fld!(13, "Readmission Indicator", "IS", O),
    fld!(14, "Admit Source", "IS", O),
    fld!(15, "Ambulatory Status", "IS", O, rep),
    fld!(16, "VIP Indicator", "IS", O),
    fld!(17, "Admitting Doctor", "XCN", O, rep),
    fld!(18, "Patient Type", "IS", O),
    fld!(19, "Visit Number", "CX", RE),
    fld!(20, "Financial Class", "FC", O, rep),
    fld!(21, "Charge Price Indicator", "IS", O),
    fld!(36, "Discharge Disposition", "IS", O, t "0112"),
    fld!(37, "Discharged to Location", "DLD", O),
    fld!(38, "Diet Type", "CE", O),
    fld!(39, "Servicing Facility", "IS", O),
    fld!(41, "Account Status", "IS", O),
    fld!(44, "Admit Date/Time", "DTM", RE),
    fld!(45, "Discharge Date/Time", "DTM", O, rep),
    fld!(50, "Alternate Visit ID", "CX", O, rep),
    fld!(51, "Visit Indicator", "IS", O, t "0326"),
];

const PV2: &[FieldSpec] = &[
    fld!(1, "Prior Pending Location", "PL", O),
    fld!(3, "Admit Reason", "CE", O),
    fld!(8, "Expected Admit Date/Time", "DTM", O),
    fld!(9, "Expected Discharge Date/Time", "DTM", O),
    fld!(23, "Clinic Organization Name", "XON", O, rep),
    fld!(24, "Patient Status Code", "IS", O),
];

const AL1: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Allergen Type Code", "CE", O, t "0127"),
    fld!(3, "Allergen Code/Mnemonic", "CE", R),
    fld!(4, "Allergy Severity Code", "CE", O, t "0128"),
    fld!(5, "Allergy Reaction Code", "ST", O, rep),
    fld!(6, "Identification Date", "DT", B),
];

const DG1: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Diagnosis Coding Method", "ID", B),
    fld!(3, "Diagnosis Code", "CE", RE),
    fld!(4, "Diagnosis Description", "ST", B),
    fld!(5, "Diagnosis Date/Time", "DTM", O),
    fld!(6, "Diagnosis Type", "IS", R, t "0052"),
    fld!(15, "Diagnosis Priority", "NM", O),
    fld!(16, "Diagnosing Clinician", "XCN", O, rep),
    fld!(19, "Attestation Date/Time", "DTM", O),
];

const PR1: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Procedure Coding Method", "IS", B),
    fld!(3, "Procedure Code", "CE", R),
    fld!(4, "Procedure Description", "ST", B),
    fld!(5, "Procedure Date/Time", "DTM", R),
    fld!(6, "Procedure Functional Type", "IS", O, t "0230"),
];

const GT1: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Guarantor Number", "CX", O, rep),
    fld!(3, "Guarantor Name", "XPN", R, rep),
    fld!(5, "Guarantor Address", "XAD", O, rep),
    fld!(6, "Guarantor Phone Number - Home", "XTN", O, rep),
    fld!(11, "Guarantor Relationship", "CE", O, t "0063"),
];

const IN1: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Insurance Plan ID", "CE", R),
    fld!(3, "Insurance Company ID", "CX", R, rep),
    fld!(4, "Insurance Company Name", "XON", RE, rep),
    fld!(5, "Insurance Company Address", "XAD", O, rep),
    fld!(8, "Group Number", "ST", O),
    fld!(12, "Plan Effective Date", "DT", O),
    fld!(13, "Plan Expiration Date", "DT", O),
    fld!(16, "Name of Insured", "XPN", O, rep),
    fld!(17, "Insured's Relationship to Patient", "CE", O, t "0063"),
    fld!(36, "Policy Number", "ST", O),
];

const ORC: &[FieldSpec] = &[
    fld!(1, "Order Control", "ID", R, t "0119"),
    fld!(2, "Placer Order Number", "EI", RE),
    fld!(3, "Filler Order Number", "EI", RE),
    fld!(4, "Placer Group Number", "EI", O),
    fld!(5, "Order Status", "ID", O, t "0038"),
    fld!(7, "Quantity/Timing", "TQ", B),
    fld!(9, "Date/Time of Transaction", "DTM", O),
    fld!(10, "Entered By", "XCN", O, rep),
    fld!(12, "Ordering Provider", "XCN", RE, rep),
    fld!(15, "Order Effective Date/Time", "DTM", O),
    fld!(16, "Order Control Code Reason", "CE", O),
    fld!(17, "Entering Organization", "CE", O),
];

const OBR: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", O),
    fld!(2, "Placer Order Number", "EI", RE),
    fld!(3, "Filler Order Number", "EI", RE),
    fld!(4, "Universal Service Identifier", "CE", R),
    fld!(6, "Requested Date/Time", "DTM", B),
    fld!(7, "Observation Date/Time", "DTM", RE),
    fld!(8, "Observation End Date/Time", "DTM", O),
    fld!(11, "Specimen Action Code", "ID", O, t "0065"),
    fld!(13, "Relevant Clinical Information", "ST", O),
    fld!(14, "Specimen Received Date/Time", "DTM", O),
    fld!(16, "Ordering Provider", "XCN", RE, rep),
    fld!(18, "Placer Field 1", "ST", O),
    fld!(19, "Placer Field 2", "ST", O),
    fld!(20, "Filler Field 1", "ST", O),
    fld!(22, "Results Rpt/Status Change Date/Time", "DTM", O),
    fld!(24, "Diagnostic Service Section ID", "ID", O, t "0074"),
    fld!(25, "Result Status", "ID", RE, t "0123"),
    fld!(27, "Quantity/Timing", "TQ", B, rep),
    fld!(31, "Reason for Study", "CE", O, rep),
];

const OBX: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", O),
    fld!(2, "Value Type", "ID", RE, t "0125"),
    fld!(3, "Observation Identifier", "CE", R),
    fld!(4, "Observation Sub-ID", "ST", O),
    fld!(5, "Observation Value", "VARIES", RE, rep),
    fld!(6, "Units", "CE", O),
    fld!(7, "References Range", "ST", O),
    fld!(8, "Abnormal Flags", "IS", O, t "0078", rep),
    fld!(9, "Probability", "NM", O),
    fld!(10, "Nature of Abnormal Test", "ID", O),
    fld!(11, "Observation Result Status", "ID", R, t "0085"),
    fld!(12, "Effective Date of Reference Range", "DTM", O),
    fld!(13, "User Defined Access Checks", "ST", O),
    fld!(14, "Date/Time of the Observation", "DTM", RE),
    fld!(15, "Producer's ID", "CE", O),
    fld!(16, "Responsible Observer", "XCN", O, rep),
    fld!(17, "Observation Method", "CE", O, rep),
    fld!(18, "Equipment Instance Identifier", "EI", O, rep),
    fld!(19, "Date/Time of the Analysis", "DTM", O),
];

const NTE: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", O),
    fld!(2, "Source of Comment", "ID", O, t "0105"),
    fld!(3, "Comment", "FT", RE, rep),
    fld!(4, "Comment Type", "CE", O),
];

const SCH: &[FieldSpec] = &[
    fld!(1, "Placer Appointment ID", "EI", RE),
    fld!(2, "Filler Appointment ID", "EI", RE),
    fld!(6, "Event Reason", "CE", O),
    fld!(7, "Appointment Reason", "CE", O),
    fld!(8, "Appointment Type", "CE", O),
    fld!(9, "Appointment Duration", "NM", O),
    fld!(10, "Appointment Duration Units", "CE", O),
    fld!(11, "Appointment Timing Quantity", "TQ", R, rep),
    fld!(16, "Filler Contact Person", "XCN", O, rep),
    fld!(20, "Entered By Person", "XCN", RE, rep),
    fld!(25, "Filler Status Code", "CE", RE),
];

const RGS: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Segment Action Code", "ID", O),
    fld!(3, "Resource Group ID", "CE", O),
];

const AIL: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(3, "Location Resource ID", "PL", RE, rep),
    fld!(4, "Location Type", "CE", RE),
    fld!(6, "Start Date/Time", "DTM", O),
];

const AIS: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(3, "Universal Service Identifier", "CE", R),
    fld!(4, "Start Date/Time", "DTM", O),
];

const AIP: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(3, "Personnel Resource ID", "XCN", RE, rep),
    fld!(4, "Resource Role", "CE", RE),
    fld!(6, "Start Date/Time", "DTM", O),
];

const TXA: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Document Type", "IS", R, t "0270"),
    fld!(4, "Activity Date/Time", "DTM", O),
    fld!(5, "Primary Activity Provider", "XCN", O, rep),
    fld!(12, "Unique Document Number", "EI", R),
    fld!(17, "Document Completion Status", "ID", R, t "0271"),
    fld!(18, "Document Confidentiality Status", "ID", O),
];

const RXA: &[FieldSpec] = &[
    fld!(1, "Give Sub-ID Counter", "NM", R),
    fld!(2, "Administration Sub-ID Counter", "NM", R),
    fld!(3, "Date/Time Start of Administration", "DTM", R),
    fld!(5, "Administered Code", "CE", R),
    fld!(6, "Administered Amount", "NM", R),
    fld!(7, "Administered Units", "CE", O),
    fld!(9, "Administration Notes", "CE", O, rep),
    fld!(20, "Completion Status", "ID", O, t "0322"),
    fld!(21, "Action Code", "ID", O, t "0323"),
];

const FT1: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", O),
    fld!(4, "Transaction Date", "DTM", R),
    fld!(6, "Transaction Type", "IS", R, t "0017"),
    fld!(7, "Transaction Code", "CE", RE),
    fld!(10, "Transaction Quantity", "NM", O),
    fld!(11, "Transaction Amount - Extended", "CP", O),
];

const IAM: &[FieldSpec] = &[
    fld!(1, "Set ID", "SI", R),
    fld!(2, "Allergen Type Code", "CE", O, t "0127"),
    fld!(3, "Allergen Code/Mnemonic", "CE", R),
    fld!(4, "Allergy Severity Code", "CE", O, t "0128"),
    fld!(8, "Action Code", "ID", R, t "0206"),
];

const QRD: &[FieldSpec] = &[
    fld!(1, "Query Date/Time", "DTM", R),
    fld!(2, "Query Format Code", "ID", R, t "0106"),
    fld!(3, "Query Priority", "ID", R, t "0091"),
    fld!(4, "Query ID", "ST", R),
    fld!(7, "Quantity Limited Request", "CQ", R),
    fld!(8, "Who Subject Filter", "XCN", R, rep),
    fld!(9, "What Subject Filter", "CE", R, rep),
    fld!(10, "What Department Data Code", "CE", R, rep),
];

const SEGMENTS: &[SegmentSpec] = &[
    SegmentSpec {
        name: "MSH",
        desc: "Message Header",
        fields: MSH,
    },
    SegmentSpec {
        name: "MSA",
        desc: "Message Acknowledgment",
        fields: MSA,
    },
    SegmentSpec {
        name: "ERR",
        desc: "Error",
        fields: ERR,
    },
    SegmentSpec {
        name: "EVN",
        desc: "Event Type",
        fields: EVN,
    },
    SegmentSpec {
        name: "PID",
        desc: "Patient Identification",
        fields: PID,
    },
    SegmentSpec {
        name: "PD1",
        desc: "Patient Additional Demographic",
        fields: PD1,
    },
    SegmentSpec {
        name: "NK1",
        desc: "Next of Kin / Associated Parties",
        fields: NK1,
    },
    SegmentSpec {
        name: "PV1",
        desc: "Patient Visit",
        fields: PV1,
    },
    SegmentSpec {
        name: "PV2",
        desc: "Patient Visit - Additional Info",
        fields: PV2,
    },
    SegmentSpec {
        name: "AL1",
        desc: "Patient Allergy Information",
        fields: AL1,
    },
    SegmentSpec {
        name: "IAM",
        desc: "Patient Adverse Reaction Information",
        fields: IAM,
    },
    SegmentSpec {
        name: "DG1",
        desc: "Diagnosis",
        fields: DG1,
    },
    SegmentSpec {
        name: "PR1",
        desc: "Procedures",
        fields: PR1,
    },
    SegmentSpec {
        name: "GT1",
        desc: "Guarantor",
        fields: GT1,
    },
    SegmentSpec {
        name: "IN1",
        desc: "Insurance",
        fields: IN1,
    },
    SegmentSpec {
        name: "ORC",
        desc: "Common Order",
        fields: ORC,
    },
    SegmentSpec {
        name: "OBR",
        desc: "Observation Request",
        fields: OBR,
    },
    SegmentSpec {
        name: "OBX",
        desc: "Observation/Result",
        fields: OBX,
    },
    SegmentSpec {
        name: "NTE",
        desc: "Notes and Comments",
        fields: NTE,
    },
    SegmentSpec {
        name: "SCH",
        desc: "Scheduling Activity Information",
        fields: SCH,
    },
    SegmentSpec {
        name: "RGS",
        desc: "Resource Group",
        fields: RGS,
    },
    SegmentSpec {
        name: "AIS",
        desc: "Appointment Information - Service",
        fields: AIS,
    },
    SegmentSpec {
        name: "AIL",
        desc: "Appointment Information - Location",
        fields: AIL,
    },
    SegmentSpec {
        name: "AIP",
        desc: "Appointment Information - Personnel",
        fields: AIP,
    },
    SegmentSpec {
        name: "TXA",
        desc: "Transcription Document Header",
        fields: TXA,
    },
    SegmentSpec {
        name: "RXA",
        desc: "Pharmacy/Treatment Administration",
        fields: RXA,
    },
    SegmentSpec {
        name: "FT1",
        desc: "Financial Transaction",
        fields: FT1,
    },
    SegmentSpec {
        name: "QRD",
        desc: "Original-Style Query Definition",
        fields: QRD,
    },
];

/// Segments recognised by name only; they pass structural checks but have no
/// field-level dictionary.
const KNOWN_SEGMENT_NAMES: &[(&str, &str)] = &[
    ("SFT", "Software Segment"),
    ("UAC", "User Authentication Credential"),
    ("ACC", "Accident"),
    ("UB1", "UB82 Data"),
    ("UB2", "UB92 Data"),
    ("DB1", "Disability"),
    ("DRG", "Diagnosis Related Group"),
    ("ROL", "Role"),
    ("PDA", "Patient Death and Autopsy"),
    ("ARV", "Access Restrictions"),
    ("MRG", "Merge Patient Information"),
    ("SPM", "Specimen"),
    ("TQ1", "Timing/Quantity"),
    ("TQ2", "Timing/Quantity Relationship"),
    ("RXE", "Pharmacy/Treatment Encoded Order"),
    ("RXR", "Pharmacy/Treatment Route"),
    ("RXC", "Pharmacy/Treatment Component Order"),
    ("RXD", "Pharmacy/Treatment Dispense"),
    ("RXG", "Pharmacy/Treatment Give"),
    ("ORO", "Order Other"),
    ("BLG", "Billing"),
    ("CTI", "Clinical Trial Identification"),
    ("QAK", "Query Acknowledgment"),
    ("QPD", "Query Parameter Definition"),
    ("RCP", "Response Control Parameter"),
    ("DSC", "Continuation Pointer"),
    ("QRF", "Original Style Query Filter"),
    ("URD", "Results/Update Definition"),
    ("URS", "Unsolicited Selection"),
    ("PRA", "Practitioner Detail"),
    ("STF", "Staff Identification"),
    ("MFI", "Master File Identification"),
    ("MFE", "Master File Entry"),
    ("LOC", "Location Identification"),
    ("NPU", "Bed Status Update"),
    ("PEO", "Product Experience Observation"),
    ("IN2", "Insurance Additional Information"),
    ("IN3", "Insurance Additional Info - Cert"),
    ("APR", "Appointment Preferences"),
    ("ARQ", "Appointment Request"),
    ("AIG", "Appointment Information - General Resource"),
    ("OBS", "Observation"),
];

pub fn segment_spec(name: &str) -> Option<&'static SegmentSpec> {
    SEGMENTS.iter().find(|s| s.name == name)
}

pub fn segment_desc(name: &str) -> Option<&'static str> {
    segment_spec(name).map(|s| s.desc).or_else(|| {
        KNOWN_SEGMENT_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, d)| *d)
    })
}

pub fn field_spec(segment: &str, seq: usize) -> Option<&'static FieldSpec> {
    segment_spec(segment).and_then(|s| s.fields.iter().find(|f| f.seq == seq))
}

/// Human label for `SEG-n`, falling back to a positional name.
pub fn field_label(segment: &str, seq: usize) -> String {
    field_spec(segment, seq).map_or_else(|| format!("Field {seq}"), |f| f.name.to_string())
}

// ---------------------------------------------------------------- code tables

pub struct TableDef {
    pub id: &'static str,
    pub name: &'static str,
    pub codes: &'static [(&'static str, &'static str)],
    /// When true an unlisted code is an error rather than a warning; reserved
    /// for tables where local extensions are not permitted.
    pub closed: bool,
}

const TABLES: &[TableDef] = &[
    TableDef {
        id: "0001",
        name: "Administrative Sex",
        closed: false,
        codes: &[
            ("F", "Female"),
            ("M", "Male"),
            ("O", "Other"),
            ("U", "Unknown"),
            ("A", "Ambiguous"),
            ("N", "Not applicable"),
        ],
    },
    TableDef {
        id: "0002",
        name: "Marital Status",
        closed: false,
        codes: &[
            ("A", "Separated"),
            ("D", "Divorced"),
            ("M", "Married"),
            ("S", "Single"),
            ("W", "Widowed"),
            ("C", "Common law"),
            ("G", "Living together"),
            ("P", "Domestic partner"),
            ("U", "Unknown"),
            ("N", "Annulled"),
            ("I", "Interlocutory"),
            ("B", "Unmarried"),
            ("T", "Unreported"),
            ("R", "Registered domestic partner"),
            ("E", "Legally separated"),
            ("O", "Other"),
        ],
    },
    TableDef {
        id: "0003",
        name: "Event Type",
        closed: false,
        codes: &[
            ("A01", "Admit/visit notification"),
            ("A02", "Transfer a patient"),
            ("A03", "Discharge/end visit"),
            ("A04", "Register a patient"),
            ("A05", "Pre-admit a patient"),
            ("A06", "Change outpatient to inpatient"),
            ("A07", "Change inpatient to outpatient"),
            ("A08", "Update patient information"),
            ("A09", "Patient departing - tracking"),
            ("A10", "Patient arriving - tracking"),
            ("A11", "Cancel admit/visit notification"),
            ("A12", "Cancel transfer"),
            ("A13", "Cancel discharge/end visit"),
            ("A14", "Pending admit"),
            ("A15", "Pending transfer"),
            ("A16", "Pending discharge"),
            ("A17", "Swap patients"),
            ("A18", "Merge patient information"),
            ("A19", "Patient query"),
            ("A20", "Bed status update"),
            ("A21", "Patient goes on leave of absence"),
            ("A22", "Patient returns from leave"),
            ("A23", "Delete a patient record"),
            ("A24", "Link patient information"),
            ("A25", "Cancel pending discharge"),
            ("A26", "Cancel pending transfer"),
            ("A27", "Cancel pending admit"),
            ("A28", "Add person information"),
            ("A29", "Delete person information"),
            ("A30", "Merge person information"),
            ("A31", "Update person information"),
            ("A32", "Cancel patient arriving"),
            ("A33", "Cancel patient departing"),
            ("A34", "Merge patient - patient ID"),
            ("A35", "Merge patient - account number"),
            ("A36", "Merge patient - ID and account"),
            ("A37", "Unlink patient information"),
            ("A38", "Cancel pre-admit"),
            ("A39", "Merge person - external ID"),
            ("A40", "Merge patient - identifier list"),
            ("A41", "Merge account - patient account number"),
            ("A44", "Move account information"),
            ("A45", "Move visit information"),
            ("A47", "Change patient identifier list"),
            ("A60", "Update allergy information"),
            ("A61", "Change consulting doctor"),
        ],
    },
    TableDef {
        id: "0004",
        name: "Patient Class",
        closed: false,
        codes: &[
            ("E", "Emergency"),
            ("I", "Inpatient"),
            ("O", "Outpatient"),
            ("P", "Preadmit"),
            ("R", "Recurring patient"),
            ("B", "Obstetrics"),
            ("C", "Commercial account"),
            ("N", "Not applicable"),
            ("U", "Unknown"),
        ],
    },
    TableDef {
        id: "0007",
        name: "Admission Type",
        closed: false,
        codes: &[
            ("A", "Accident"),
            ("C", "Elective"),
            ("E", "Emergency"),
            ("L", "Labor and delivery"),
            ("N", "Newborn"),
            ("R", "Routine"),
            ("U", "Urgent"),
        ],
    },
    TableDef {
        id: "0008",
        name: "Acknowledgment Code",
        closed: true,
        codes: &[
            ("AA", "Original mode: application accept"),
            ("AE", "Original mode: application error"),
            ("AR", "Original mode: application reject"),
            ("CA", "Enhanced mode: commit accept"),
            ("CE", "Enhanced mode: commit error"),
            ("CR", "Enhanced mode: commit reject"),
        ],
    },
    TableDef {
        id: "0017",
        name: "Transaction Type",
        closed: false,
        codes: &[
            ("CG", "Charge"),
            ("CD", "Credit"),
            ("PY", "Payment"),
            ("AJ", "Adjustment"),
        ],
    },
    TableDef {
        id: "0038",
        name: "Order Status",
        closed: false,
        codes: &[
            ("A", "Some results available"),
            ("CA", "Order was cancelled"),
            ("CM", "Order is completed"),
            ("DC", "Order was discontinued"),
            ("ER", "Error, order not found"),
            ("HD", "Order is on hold"),
            ("IP", "In process, unspecified"),
            ("RP", "Order has been replaced"),
            ("SC", "In process, scheduled"),
        ],
    },
    TableDef {
        id: "0052",
        name: "Diagnosis Type",
        closed: false,
        codes: &[("A", "Admitting"), ("W", "Working"), ("F", "Final")],
    },
    TableDef {
        id: "0063",
        name: "Relationship",
        closed: false,
        codes: &[
            ("SEL", "Self"),
            ("SPO", "Spouse"),
            ("DOM", "Life partner"),
            ("CHD", "Child"),
            ("PAR", "Parent"),
            ("MTH", "Mother"),
            ("FTH", "Father"),
            ("SIB", "Sibling"),
            ("BRO", "Brother"),
            ("SIS", "Sister"),
            ("GRD", "Guardian"),
            ("EMR", "Emergency contact"),
            ("FND", "Friend"),
            ("OTH", "Other"),
            ("UNK", "Unknown"),
            ("EME", "Employee"),
            ("EMC", "Emergency contact"),
            ("GRP", "Grandparent"),
            ("EXF", "Extended family"),
        ],
    },
    TableDef {
        id: "0065",
        name: "Specimen Action Code",
        closed: false,
        codes: &[
            ("A", "Add ordered tests"),
            ("G", "Generated order"),
            ("L", "Lab to obtain specimen"),
            ("O", "Specimen obtained by service other than lab"),
            ("P", "Pending specimen"),
            ("R", "Revised order"),
            ("S", "Schedule the tests"),
        ],
    },
    TableDef {
        id: "0074",
        name: "Diagnostic Service Section ID",
        closed: false,
        codes: &[
            ("AU", "Audiology"),
            ("BG", "Blood gases"),
            ("BLB", "Blood bank"),
            ("CH", "Chemistry"),
            ("CP", "Cytopathology"),
            ("CT", "CAT scan"),
            ("CTH", "Cardiac catheterization"),
            ("CUS", "Cardiac ultrasound"),
            ("EC", "Electrocardiac"),
            ("EN", "Electroneuro"),
            ("HM", "Hematology"),
            ("ICU", "Bedside ICU monitoring"),
            ("IMM", "Immunology"),
            ("LAB", "Laboratory"),
            ("MB", "Microbiology"),
            ("MCB", "Mycobacteriology"),
            ("MYC", "Mycology"),
            ("NMR", "Nuclear magnetic resonance"),
            ("NMS", "Nuclear medicine scan"),
            ("NRS", "Nursing service measures"),
            ("OSL", "Outside lab"),
            ("OT", "Occupational therapy"),
            ("OTH", "Other"),
            ("OUS", "OB ultrasound"),
            ("PF", "Pulmonary function"),
            ("PHR", "Pharmacy"),
            ("PHY", "Physician (Hx, Dx, admission note)"),
            ("PT", "Physical therapy"),
            ("RAD", "Radiology"),
            ("RC", "Respiratory care"),
            ("RT", "Radiation therapy"),
            ("RUS", "Radiology ultrasound"),
            ("RX", "Radiograph"),
            ("SP", "Surgical pathology"),
            ("SR", "Serology"),
            ("TX", "Toxicology"),
            ("VR", "Virology"),
            ("VUS", "Vascular ultrasound"),
            ("XRC", "Cineradiograph"),
        ],
    },
    TableDef {
        id: "0076",
        name: "Message Type",
        closed: false,
        codes: &[
            ("ACK", "General acknowledgment"),
            ("ADT", "Admit/discharge/transfer"),
            ("BAR", "Add/change billing account"),
            ("DFT", "Detail financial transaction"),
            ("MDM", "Medical document management"),
            ("MFN", "Master files notification"),
            ("MFK", "Master files application acknowledgment"),
            ("OMG", "General clinical order"),
            ("OML", "Laboratory order"),
            ("OMP", "Pharmacy/treatment order"),
            ("ORM", "Order message"),
            ("ORR", "Order response"),
            ("ORU", "Observation result unsolicited"),
            ("OUL", "Unsolicited laboratory observation"),
            ("QBP", "Query by parameter"),
            ("QRY", "Query, original mode"),
            ("RAS", "Pharmacy/treatment administration"),
            ("RDE", "Pharmacy/treatment encoded order"),
            ("RDS", "Pharmacy/treatment dispense"),
            ("RGV", "Pharmacy/treatment give"),
            ("RSP", "Segment pattern response"),
            ("SIU", "Schedule information unsolicited"),
            ("SRM", "Schedule request"),
            ("SRR", "Scheduled request response"),
            ("SQM", "Schedule query"),
            ("VXU", "Unsolicited vaccination record update"),
            ("VXQ", "Query for vaccination record"),
        ],
    },
    TableDef {
        id: "0078",
        name: "Abnormal Flags",
        closed: false,
        codes: &[
            ("L", "Below low normal"),
            ("H", "Above high normal"),
            ("LL", "Below lower panic limit"),
            ("HH", "Above upper panic limit"),
            ("N", "Normal"),
            ("A", "Abnormal"),
            ("AA", "Critically abnormal"),
            ("<", "Below absolute low-off instrument scale"),
            (">", "Above absolute high-off instrument scale"),
            ("S", "Susceptible"),
            ("R", "Resistant"),
            ("I", "Intermediate"),
            ("MS", "Moderately susceptible"),
            ("VS", "Very susceptible"),
            ("U", "Significant change up"),
            ("D", "Significant change down"),
            ("B", "Better"),
            ("W", "Worse"),
            ("null", "No level defined"),
        ],
    },
    TableDef {
        id: "0085",
        name: "Observation Result Status",
        closed: false,
        codes: &[
            ("C", "Corrected result"),
            ("D", "Deleted"),
            ("F", "Final result"),
            ("I", "Specimen in lab, results pending"),
            ("P", "Preliminary result"),
            ("R", "Results entered, not verified"),
            ("S", "Partial results"),
            ("U", "Results status change to final"),
            ("W", "Post original as wrong"),
            ("X", "Results cannot be obtained"),
        ],
    },
    TableDef {
        id: "0091",
        name: "Query Priority",
        closed: false,
        codes: &[("D", "Deferred"), ("I", "Immediate")],
    },
    TableDef {
        id: "0103",
        name: "Processing ID",
        closed: true,
        codes: &[("D", "Debugging"), ("P", "Production"), ("T", "Training")],
    },
    TableDef {
        id: "0104",
        name: "Version ID",
        closed: true,
        codes: &[
            ("2.0", "Release 2.0 (1988)"),
            ("2.0D", "Demo 2.0"),
            ("2.1", "Release 2.1 (1990)"),
            ("2.2", "Release 2.2 (1994)"),
            ("2.3", "Release 2.3 (1997)"),
            ("2.3.1", "Release 2.3.1 (1999)"),
            ("2.4", "Release 2.4 (2000)"),
            ("2.5", "Release 2.5 (2003)"),
            ("2.5.1", "Release 2.5.1 (2007)"),
            ("2.6", "Release 2.6 (2007)"),
            ("2.7", "Release 2.7 (2011)"),
            ("2.7.1", "Release 2.7.1 (2012)"),
            ("2.8", "Release 2.8 (2014)"),
            ("2.8.1", "Release 2.8.1 (2015)"),
            ("2.8.2", "Release 2.8.2 (2016)"),
            ("2.9", "Release 2.9 (2019)"),
        ],
    },
    TableDef {
        id: "0105",
        name: "Source of Comment",
        closed: false,
        codes: &[
            ("L", "Ancillary (filler) department"),
            ("P", "Orderer (placer)"),
            ("O", "Other system"),
        ],
    },
    TableDef {
        id: "0106",
        name: "Query/Response Format Code",
        closed: false,
        codes: &[
            ("D", "Response is in display format"),
            ("R", "Response is in record-oriented format"),
            ("T", "Response is in tabular format"),
        ],
    },
    TableDef {
        id: "0112",
        name: "Discharge Disposition",
        closed: false,
        codes: &[
            ("01", "Discharged to home or self care"),
            ("02", "Discharged to another short term hospital"),
            ("03", "Discharged to skilled nursing facility"),
            ("04", "Discharged to intermediate care facility"),
            ("05", "Discharged to another institution"),
            ("06", "Discharged to home under care of home health service"),
            ("07", "Left against medical advice"),
            ("20", "Expired"),
            ("30", "Still patient"),
        ],
    },
    TableDef {
        id: "0119",
        name: "Order Control Codes",
        closed: false,
        codes: &[
            ("NW", "New order"),
            ("OK", "Order accepted"),
            ("CA", "Cancel order request"),
            ("CR", "Cancelled as requested"),
            ("DC", "Discontinue order request"),
            ("DR", "Discontinued as requested"),
            ("HD", "Hold order request"),
            ("RO", "Replacement order"),
            ("RP", "Order replace request"),
            ("RU", "Replaced unsolicited"),
            ("SC", "Status changed"),
            ("SN", "Send order number"),
            ("UA", "Unable to accept order"),
            ("UC", "Unable to cancel"),
            ("XO", "Change order request"),
            ("XX", "Order changed, unsolicited"),
            ("RE", "Observations to follow"),
        ],
    },
    TableDef {
        id: "0123",
        name: "Result Status",
        closed: false,
        codes: &[
            ("O", "Order received, specimen not yet received"),
            ("I", "No results available, specimen received"),
            ("S", "No results available, procedure scheduled"),
            ("A", "Some results available"),
            ("P", "Preliminary"),
            ("C", "Correction to results"),
            ("R", "Results entered, not verified"),
            ("F", "Final results"),
            ("X", "No results available, order cancelled"),
            ("Y", "No order on record"),
            ("Z", "No record of this patient"),
        ],
    },
    TableDef {
        id: "0125",
        name: "Value Type",
        closed: false,
        codes: &[
            ("AD", "Address"),
            ("CE", "Coded entry"),
            ("CF", "Coded element with formatted values"),
            ("CK", "Composite ID with check digit"),
            ("CN", "Composite ID and name"),
            ("CP", "Composite price"),
            ("CWE", "Coded with exceptions"),
            ("CX", "Extended composite ID"),
            ("DT", "Date"),
            ("ED", "Encapsulated data"),
            ("FT", "Formatted text"),
            ("ID", "Coded value"),
            ("IS", "Coded value, user table"),
            ("MO", "Money"),
            ("NM", "Numeric"),
            ("PN", "Person name"),
            ("RP", "Reference pointer"),
            ("SN", "Structured numeric"),
            ("ST", "String"),
            ("TM", "Time"),
            ("TN", "Telephone number"),
            ("TS", "Time stamp"),
            ("TX", "Text"),
            ("XAD", "Extended address"),
            ("XCN", "Extended composite name and ID"),
            ("XON", "Extended organization name"),
            ("XPN", "Extended person name"),
            ("XTN", "Extended telecommunication number"),
        ],
    },
    TableDef {
        id: "0127",
        name: "Allergen Type",
        closed: false,
        codes: &[
            ("DA", "Drug allergy"),
            ("FA", "Food allergy"),
            ("MA", "Miscellaneous allergy"),
            ("MC", "Miscellaneous contraindication"),
            ("EA", "Environmental allergy"),
            ("AA", "Animal allergy"),
            ("PA", "Plant allergy"),
            ("LA", "Pollen allergy"),
        ],
    },
    TableDef {
        id: "0128",
        name: "Allergy Severity",
        closed: false,
        codes: &[
            ("SV", "Severe"),
            ("MO", "Moderate"),
            ("MI", "Mild"),
            ("U", "Unknown"),
        ],
    },
    TableDef {
        id: "0136",
        name: "Yes/No Indicator",
        closed: true,
        codes: &[("Y", "Yes"), ("N", "No")],
    },
    TableDef {
        id: "0155",
        name: "Accept/Application Acknowledgment Conditions",
        closed: true,
        codes: &[
            ("AL", "Always"),
            ("NE", "Never"),
            ("ER", "Only on error"),
            ("SU", "Only on success"),
        ],
    },
    TableDef {
        id: "0206",
        name: "Segment Action Code",
        closed: true,
        codes: &[("A", "Add/insert"), ("D", "Delete"), ("U", "Update")],
    },
    TableDef {
        id: "0230",
        name: "Procedure Functional Type",
        closed: false,
        codes: &[
            ("A", "Anesthesia"),
            ("D", "Diagnostic procedure"),
            ("I", "Invasive procedure not classified"),
            ("P", "Procedure for treatment"),
        ],
    },
    TableDef {
        id: "0270",
        name: "Document Type",
        closed: false,
        codes: &[
            ("AR", "Autopsy report"),
            ("CD", "Cardiodiagnostics"),
            ("CN", "Consultation"),
            ("DS", "Discharge summary"),
            ("ED", "Emergency department report"),
            ("HP", "History and physical examination"),
            ("OP", "Operative report"),
            ("PC", "Psychiatric consultation"),
            ("PH", "Psychiatric history and physical"),
            ("PN", "Procedure note"),
            ("PR", "Progress note"),
            ("SP", "Surgical pathology"),
            ("TS", "Transfer summary"),
        ],
    },
    TableDef {
        id: "0271",
        name: "Document Completion Status",
        closed: false,
        codes: &[
            ("AU", "Authenticated"),
            ("DI", "Dictated"),
            ("DO", "Documented"),
            ("IN", "Incomplete"),
            ("IP", "In progress"),
            ("LA", "Legally authenticated"),
            ("PA", "Pre-authenticated"),
        ],
    },
    TableDef {
        id: "0322",
        name: "Completion Status",
        closed: false,
        codes: &[
            ("CP", "Complete"),
            ("RE", "Refused"),
            ("NA", "Not administered"),
            ("PA", "Partially administered"),
        ],
    },
    TableDef {
        id: "0323",
        name: "Action Code",
        closed: false,
        codes: &[("A", "Add"), ("D", "Delete"), ("U", "Update")],
    },
    TableDef {
        id: "0326",
        name: "Visit Indicator",
        closed: false,
        codes: &[("A", "Account level"), ("V", "Visit level")],
    },
    TableDef {
        id: "0357",
        name: "Message Error Condition Codes",
        closed: false,
        codes: &[
            ("0", "Message accepted"),
            ("100", "Segment sequence error"),
            ("101", "Required field missing"),
            ("102", "Data type error"),
            ("103", "Table value not found"),
            ("200", "Unsupported message type"),
            ("201", "Unsupported event code"),
            ("202", "Unsupported processing id"),
            ("203", "Unsupported version id"),
            ("204", "Unknown key identifier"),
            ("205", "Duplicate key identifier"),
            ("206", "Application record locked"),
            ("207", "Application internal error"),
        ],
    },
    TableDef {
        id: "0516",
        name: "Error Severity",
        closed: true,
        codes: &[("E", "Error"), ("W", "Warning"), ("I", "Information")],
    },
];

pub fn table(id: &str) -> Option<&'static TableDef> {
    TABLES.iter().find(|t| t.id == id)
}

impl TableDef {
    pub fn meaning(&self, code: &str) -> Option<&'static str> {
        self.codes.iter().find(|(c, _)| *c == code).map(|(_, m)| *m)
    }
}

/// Decoded meaning for a value in a named table, if the table is known.
pub fn code_meaning(table_id: &str, code: &str) -> Option<&'static str> {
    table(table_id).and_then(|t| t.meaning(code))
}

// ----------------------------------------------------------- message grammar

#[derive(Debug, Clone, Copy)]
pub struct StructSeg {
    pub name: &'static str,
    pub required: bool,
}

const fn req(name: &'static str) -> StructSeg {
    StructSeg {
        name,
        required: true,
    }
}
const fn opt(name: &'static str) -> StructSeg {
    StructSeg {
        name,
        required: false,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MessageSpec {
    pub id: &'static str,
    pub desc: &'static str,
    /// Expected segments in abstract-message order.
    pub segments: &'static [StructSeg],
}

const ADT_VISIT: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("EVN"),
    req("PID"),
    opt("PD1"),
    opt("ARV"),
    opt("ROL"),
    opt("NK1"),
    req("PV1"),
    opt("PV2"),
    opt("ROL"),
    opt("DB1"),
    opt("OBX"),
    opt("AL1"),
    opt("DG1"),
    opt("DRG"),
    opt("PR1"),
    opt("GT1"),
    opt("IN1"),
    opt("IN2"),
    opt("IN3"),
    opt("ACC"),
    opt("UB1"),
    opt("UB2"),
    opt("PDA"),
];

const ADT_PERSON: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("EVN"),
    req("PID"),
    opt("PD1"),
    opt("ARV"),
    opt("ROL"),
    opt("NK1"),
    opt("PV1"),
    opt("PV2"),
    opt("DB1"),
    opt("OBX"),
    opt("AL1"),
    opt("DG1"),
    opt("DRG"),
    opt("PR1"),
    opt("GT1"),
    opt("IN1"),
    opt("IN2"),
    opt("IN3"),
    opt("ACC"),
    opt("UB1"),
    opt("UB2"),
];

const ADT_MERGE: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("EVN"),
    req("PID"),
    opt("PD1"),
    req("MRG"),
    opt("PV1"),
];

const ORU_R01: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    opt("PID"),
    opt("PD1"),
    opt("NTE"),
    opt("NK1"),
    opt("PV1"),
    opt("PV2"),
    opt("ORC"),
    req("OBR"),
    opt("NTE"),
    opt("TQ1"),
    opt("CTD"),
    opt("OBX"),
    opt("NTE"),
    opt("SPM"),
    opt("DSC"),
];

const ORM_O01: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    opt("PID"),
    opt("PD1"),
    opt("NTE"),
    opt("PV1"),
    opt("PV2"),
    opt("IN1"),
    opt("IN2"),
    opt("IN3"),
    opt("GT1"),
    opt("AL1"),
    req("ORC"),
    opt("OBR"),
    opt("RXO"),
    opt("RQD"),
    opt("ODS"),
    opt("NTE"),
    opt("DG1"),
    opt("OBX"),
    opt("NTE"),
    opt("CTI"),
    opt("BLG"),
];

const OML_O21: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    opt("NTE"),
    opt("PID"),
    opt("PD1"),
    opt("NTE"),
    opt("PV1"),
    opt("PV2"),
    opt("IN1"),
    opt("GT1"),
    opt("AL1"),
    req("ORC"),
    opt("TQ1"),
    opt("TQ2"),
    req("OBR"),
    opt("NTE"),
    opt("DG1"),
    opt("OBX"),
    opt("NTE"),
    opt("SPM"),
];

const ACK_MSG: &[StructSeg] = &[req("MSH"), opt("SFT"), req("MSA"), opt("ERR")];

const SIU_MSG: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("SCH"),
    opt("TQ1"),
    opt("NTE"),
    opt("PID"),
    opt("PD1"),
    opt("PV1"),
    opt("PV2"),
    opt("OBX"),
    opt("DG1"),
    opt("RGS"),
    opt("AIS"),
    opt("AIG"),
    opt("AIL"),
    opt("AIP"),
    opt("NTE"),
];

const MDM_T02: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("EVN"),
    req("PID"),
    req("PV1"),
    opt("ORC"),
    opt("OBR"),
    opt("NTE"),
    req("TXA"),
    opt("OBX"),
    opt("NTE"),
];

const MDM_T01: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("EVN"),
    req("PID"),
    req("PV1"),
    req("TXA"),
];

const VXU_V04: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("PID"),
    opt("PD1"),
    opt("NK1"),
    opt("PV1"),
    opt("PV2"),
    opt("GT1"),
    opt("IN1"),
    opt("IN2"),
    opt("IN3"),
    opt("ORC"),
    opt("TQ1"),
    req("RXA"),
    opt("RXR"),
    opt("OBX"),
    opt("NTE"),
];

const DFT_P03: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("EVN"),
    req("PID"),
    opt("PD1"),
    opt("ROL"),
    opt("PV1"),
    opt("PV2"),
    opt("ROL"),
    opt("DB1"),
    opt("OBX"),
    opt("DG1"),
    opt("DRG"),
    opt("GT1"),
    opt("IN1"),
    opt("IN2"),
    opt("IN3"),
    opt("ACC"),
    req("FT1"),
    opt("NTE"),
    opt("PR1"),
];

const BAR_P01: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    req("EVN"),
    req("PID"),
    opt("PD1"),
    opt("PV1"),
    opt("PV2"),
    opt("DB1"),
    opt("OBX"),
    opt("AL1"),
    opt("DG1"),
    opt("DRG"),
    opt("PR1"),
    opt("GT1"),
    opt("NK1"),
    opt("IN1"),
    opt("IN2"),
    opt("IN3"),
    opt("ACC"),
    opt("UB1"),
    opt("UB2"),
];

const QRY_MSG: &[StructSeg] = &[req("MSH"), req("QRD"), opt("QRF"), opt("DSC")];

const RDE_MSG: &[StructSeg] = &[
    req("MSH"),
    opt("SFT"),
    opt("NTE"),
    opt("PID"),
    opt("PD1"),
    opt("NTE"),
    opt("PV1"),
    opt("PV2"),
    opt("IN1"),
    opt("GT1"),
    opt("AL1"),
    req("ORC"),
    opt("TQ1"),
    req("RXE"),
    opt("NTE"),
    opt("RXR"),
    opt("RXC"),
    opt("OBX"),
    opt("CTI"),
];

const MESSAGES: &[MessageSpec] = &[
    MessageSpec {
        id: "ADT^A01",
        desc: "Admit / Visit Notification",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A02",
        desc: "Transfer a Patient",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A03",
        desc: "Discharge / End Visit",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A04",
        desc: "Register a Patient",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A05",
        desc: "Pre-admit a Patient",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A06",
        desc: "Change Outpatient to Inpatient",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A07",
        desc: "Change Inpatient to Outpatient",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A08",
        desc: "Update Patient Information",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A09",
        desc: "Patient Departing - Tracking",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A10",
        desc: "Patient Arriving - Tracking",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A11",
        desc: "Cancel Admit / Visit Notification",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A12",
        desc: "Cancel Transfer",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A13",
        desc: "Cancel Discharge / End Visit",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A14",
        desc: "Pending Admit",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A15",
        desc: "Pending Transfer",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A16",
        desc: "Pending Discharge",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A17",
        desc: "Swap Patients",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A18",
        desc: "Merge Patient Information",
        segments: ADT_MERGE,
    },
    MessageSpec {
        id: "ADT^A20",
        desc: "Bed Status Update",
        segments: &[req("MSH"), req("EVN"), req("NPU")],
    },
    MessageSpec {
        id: "ADT^A21",
        desc: "Patient Goes on Leave of Absence",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A22",
        desc: "Patient Returns from Leave of Absence",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A23",
        desc: "Delete a Patient Record",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A25",
        desc: "Cancel Pending Discharge",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A26",
        desc: "Cancel Pending Transfer",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A27",
        desc: "Cancel Pending Admit",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A28",
        desc: "Add Person Information",
        segments: ADT_PERSON,
    },
    MessageSpec {
        id: "ADT^A29",
        desc: "Delete Person Information",
        segments: ADT_PERSON,
    },
    MessageSpec {
        id: "ADT^A30",
        desc: "Merge Person Information",
        segments: ADT_MERGE,
    },
    MessageSpec {
        id: "ADT^A31",
        desc: "Update Person Information",
        segments: ADT_PERSON,
    },
    MessageSpec {
        id: "ADT^A34",
        desc: "Merge Patient Information - Patient ID",
        segments: ADT_MERGE,
    },
    MessageSpec {
        id: "ADT^A35",
        desc: "Merge Patient Information - Account Number",
        segments: ADT_MERGE,
    },
    MessageSpec {
        id: "ADT^A38",
        desc: "Cancel Pre-admit",
        segments: ADT_VISIT,
    },
    MessageSpec {
        id: "ADT^A40",
        desc: "Merge Patient - Patient Identifier List",
        segments: ADT_MERGE,
    },
    MessageSpec {
        id: "ADT^A44",
        desc: "Move Account Information",
        segments: ADT_MERGE,
    },
    MessageSpec {
        id: "ADT^A45",
        desc: "Move Visit Information",
        segments: ADT_MERGE,
    },
    MessageSpec {
        id: "ADT^A47",
        desc: "Change Patient Identifier List",
        segments: ADT_MERGE,
    },
    MessageSpec {
        id: "ADT^A60",
        desc: "Update Allergy Information",
        segments: ADT_PERSON,
    },
    MessageSpec {
        id: "ACK",
        desc: "General Acknowledgment",
        segments: ACK_MSG,
    },
    MessageSpec {
        id: "ORU^R01",
        desc: "Unsolicited Observation Result",
        segments: ORU_R01,
    },
    MessageSpec {
        id: "ORU^R30",
        desc: "Unsolicited Point-of-Care Observation",
        segments: ORU_R01,
    },
    MessageSpec {
        id: "OUL^R21",
        desc: "Unsolicited Laboratory Observation",
        segments: ORU_R01,
    },
    MessageSpec {
        id: "ORM^O01",
        desc: "Order Message",
        segments: ORM_O01,
    },
    MessageSpec {
        id: "OMG^O19",
        desc: "General Clinical Order",
        segments: OML_O21,
    },
    MessageSpec {
        id: "OML^O21",
        desc: "Laboratory Order",
        segments: OML_O21,
    },
    MessageSpec {
        id: "ORR^O02",
        desc: "Order Response",
        segments: &[
            req("MSH"),
            req("MSA"),
            opt("ERR"),
            opt("PID"),
            opt("ORC"),
            opt("OBR"),
        ],
    },
    MessageSpec {
        id: "RDE^O11",
        desc: "Pharmacy/Treatment Encoded Order",
        segments: RDE_MSG,
    },
    MessageSpec {
        id: "SIU^S12",
        desc: "Notification of New Appointment",
        segments: SIU_MSG,
    },
    MessageSpec {
        id: "SIU^S13",
        desc: "Notification of Appointment Rescheduling",
        segments: SIU_MSG,
    },
    MessageSpec {
        id: "SIU^S14",
        desc: "Notification of Appointment Modification",
        segments: SIU_MSG,
    },
    MessageSpec {
        id: "SIU^S15",
        desc: "Notification of Appointment Cancellation",
        segments: SIU_MSG,
    },
    MessageSpec {
        id: "SIU^S17",
        desc: "Notification of Appointment Deletion",
        segments: SIU_MSG,
    },
    MessageSpec {
        id: "SIU^S26",
        desc: "Notification of Patient Did Not Show",
        segments: SIU_MSG,
    },
    MessageSpec {
        id: "MDM^T01",
        desc: "Original Document Notification",
        segments: MDM_T01,
    },
    MessageSpec {
        id: "MDM^T02",
        desc: "Original Document Notification and Content",
        segments: MDM_T02,
    },
    MessageSpec {
        id: "MDM^T04",
        desc: "Document Status Change Notification and Content",
        segments: MDM_T02,
    },
    MessageSpec {
        id: "MDM^T06",
        desc: "Document Addendum Notification and Content",
        segments: MDM_T02,
    },
    MessageSpec {
        id: "MDM^T08",
        desc: "Document Edit Notification and Content",
        segments: MDM_T02,
    },
    MessageSpec {
        id: "VXU^V04",
        desc: "Unsolicited Vaccination Record Update",
        segments: VXU_V04,
    },
    MessageSpec {
        id: "DFT^P03",
        desc: "Post Detail Financial Transaction",
        segments: DFT_P03,
    },
    MessageSpec {
        id: "BAR^P01",
        desc: "Add Patient Account",
        segments: BAR_P01,
    },
    MessageSpec {
        id: "BAR^P02",
        desc: "Purge Patient Account",
        segments: BAR_P01,
    },
    MessageSpec {
        id: "QRY^A19",
        desc: "Patient Query",
        segments: QRY_MSG,
    },
    MessageSpec {
        id: "QRY^Q01",
        desc: "Query Sent for Immediate Response",
        segments: QRY_MSG,
    },
];

/// Looks up the abstract structure for `code^trigger`, falling back to the bare
/// message code (ACK has no trigger event of its own in most profiles).
pub fn message_spec(code: &str, trigger: &str) -> Option<&'static MessageSpec> {
    let key = format!("{code}^{trigger}");
    MESSAGES
        .iter()
        .find(|m| m.id == key)
        .or_else(|| MESSAGES.iter().find(|m| m.id == code))
}

/// Description of a trigger event even when the full structure is unknown.
pub fn trigger_desc(trigger: &str) -> Option<&'static str> {
    code_meaning("0003", trigger)
}

/// Versions this tool knows how to reason about.
pub fn is_known_version(v: &str) -> bool {
    table("0104").is_some_and(|t| t.meaning(v).is_some())
}
