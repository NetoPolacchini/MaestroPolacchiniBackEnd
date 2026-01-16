// src/services/document_service.rs

use sqlx::{Postgres, Executor, Acquire};
use uuid::Uuid;
use genpdf::{elements, style, Element, Margins};
use image::Luma;
use qrcode::QrCode;
use crate::{
    common::error::AppError,
    db::{
        OperationsRepository,
        SettingsRepository
    },
};

#[derive(Clone)]
pub struct DocumentService {
    repo: OperationsRepository,
    settings_repo: SettingsRepository,
}

impl DocumentService {
    pub fn new(repo: OperationsRepository, settings_repo: SettingsRepository) -> Self {
        Self { repo, settings_repo }
    }

    pub async fn generate_order_pdf<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        order_id: Uuid,
    ) -> Result<Vec<u8>, AppError>
    where
        E: Executor<'e, Database = Postgres> + Acquire<'e, Database = Postgres>,
    {
        let mut tx = executor.begin().await?;

        // 1. Busca os Dados
        let order_detail = self.repo.get_order_detail(&mut *tx, tenant_id, order_id).await?;
        let items = self.repo.list_order_items(&mut *tx, tenant_id, order_id).await?;
        let settings = self.settings_repo.get_settings(&mut *tx, tenant_id).await?;

        // Precisamos dos nomes dos itens. O list_order_items retorna OrderItem (que só tem ID).
        // Vamos precisar fazer um loop ou join para pegar o nome.
        // Para simplificar, vou assumir que você vai criar um método `list_order_items_with_names` no Repo,
        // OU vamos fazer uma query rápida aqui para pegar os nomes.
        // Vou fazer a query aqui para não te travar no Repo:

        struct ItemPrintData {
            name: String,
            quantity: rust_decimal::Decimal,
            price: rust_decimal::Decimal,
            total: rust_decimal::Decimal,
        }

        let mut print_items = Vec::new();
        for item in items {
            let name_row = sqlx::query!("SELECT name FROM items WHERE id = $1", item.item_id)
                .fetch_one(&mut *tx).await?;

            print_items.push(ItemPrintData {
                name: name_row.name,
                quantity: item.quantity,
                price: item.unit_price,
                total: (item.quantity * item.unit_price) - item.discount
            });
        }

        tx.commit().await?;

        // 2. Configura o PDF
        // Carrega a fonte da pasta 'fonts/'
        let font_family = genpdf::fonts::from_files("./fonts", "Roboto", None)
            .map_err(|_| AppError::FontNotFound("Fonte não encontrada na pasta ./fonts".to_string()))?;

        let mut doc = genpdf::Document::new(font_family);
        doc.set_title(format!("Pedido #{}", order_detail.header.display_id));
        let mut decorator = genpdf::SimplePageDecorator::new();
        decorator.set_margins(10);
        doc.set_page_decorator(decorator);

        // --- CABEÇALHO DINÂMICO ---
        // Se tiver nome da empresa configurado, usa. Senão, usa o padrão.
        let title_text = settings.company_name.unwrap_or("MAESTRO ERP".to_string());
        doc.push(elements::Paragraph::new(title_text)
            .styled(style::Style::new().bold().with_font_size(18)));

        if let Some(doc_num) = settings.document_number {
            doc.push(elements::Paragraph::new(format!("CNPJ/CPF: {}", doc_num))
                .styled(style::Style::new().with_font_size(10)));
        }

        doc.push(elements::Break::new(1.5));

        doc.push(elements::Paragraph::new(format!("ORÇAMENTO / PEDIDO #{}", order_detail.header.display_id))
            .styled(style::Style::new().bold().with_font_size(14)));

        doc.push(elements::Paragraph::new(format!("Data: {}", order_detail.header.created_at.format("%d/%m/%Y"))));

        if let Some(cust) = order_detail.customer_name {
            doc.push(elements::Paragraph::new(format!("Cliente: {}", cust)));
        } else {
            doc.push(elements::Paragraph::new("Cliente: Consumidor Final"));
        }

        doc.push(elements::Break::new(2));

        // --- TABELA DE ITENS ---
        // Pesos das colunas: Nome (4), Qtd (1), Preço (2), Total (2)
        let mut table = elements::TableLayout::new(vec![4, 1, 2, 2]);
        table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

        // Header da Tabela
        let style_bold = style::Style::new().bold();
        table.row()
            .element(elements::Paragraph::new("Produto").styled(style_bold))
            .element(elements::Paragraph::new("Qtd").styled(style_bold))
            .element(elements::Paragraph::new("Unitário").styled(style_bold))
            .element(elements::Paragraph::new("Total").styled(style_bold))
            .push()
            .expect("Table error");

        // Linhas
        for item in print_items {
            table.row()
                .element(elements::Paragraph::new(item.name))
                .element(elements::Paragraph::new(format!("{:.2}", item.quantity)))
                .element(elements::Paragraph::new(format!("R$ {:.2}", item.price)))
                .element(elements::Paragraph::new(format!("R$ {:.2}", item.total)))
                .push()
                .expect("Table row error");
        }

        doc.push(table);
        doc.push(elements::Break::new(2));

        // --- TOTAIS ---
        let mut total_paragraph = elements::Paragraph::new(
            format!("TOTAL GERAL: R$ {:.2}", order_detail.header.total_amount)
        );

        total_paragraph.set_alignment(genpdf::Alignment::Right);

        doc.push(total_paragraph.styled(style::Style::new().bold().with_font_size(12)));

        doc.push(elements::Break::new(2));

        // --- ÁREA DE PAGAMENTO (QR CODE) ---
        if let Some(key) = settings.pix_key {
            doc.push(elements::Paragraph::new("PAGAMENTO VIA PIX")
                .styled(style::Style::new().bold().with_font_size(12)));

            doc.push(elements::Paragraph::new(format!("Chave: {}", key)));
            doc.push(elements::Break::new(1));

            // Gera o QR Code
            // Obs: Isso gera um QR Code simples da string da chave.
            // Para "Pix Copia e Cola" oficial (EMV BR Code), precisaríamos de uma lib específica de Payload Pix.
            // Por enquanto, assumimos que o usuário vai colocar o payload ou apenas a chave (email/cpf).
            // Se for chave simples, o QR Code será apenas o texto da chave.

            let code = QrCode::new(key.as_bytes())
                .map_err(|e| AppError::InternalServerError(anyhow::Error::msg(e.to_string())))?;

            // Renderiza para imagem
            let image_buffer = code.render::<Luma<u8>>().build();
            let dynamic_image = image::DynamicImage::ImageLuma8(image_buffer);

            // Converte para genpdf::Image
            let pdf_image = genpdf::elements::Image::from_dynamic_image(dynamic_image)
                .map_err(|e| AppError::InternalServerError(anyhow::Error::msg(e.to_string())))?
                .with_scale(genpdf::Scale::new(0.5, 0.5)); // Ajuste o tamanho conforme necessário

            doc.push(pdf_image);
        }

        // --- RODAPÉ ---
        if let Some(addr) = settings.address {
            doc.push(elements::Break::new(2));
            doc.push(elements::Paragraph::new(addr).styled(style::Style::new().italic().with_font_size(8)));
        }


        // 3. Renderiza para Buffer (Memória)
        let mut buffer = Vec::new();
        doc.render(&mut buffer)
            .map_err(|e| AppError::InternalServerError(anyhow::Error::msg(e.to_string())))?;

        Ok(buffer)
    }
}